use locdev_core::email::{EmailFormat, Segment};
use serde_json::json;

use crate::blocks::{Flavor, default_theme, emit, parse};
use crate::generate::{Kind, plan, starter};

fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (path, text) in files {
        let p = dir.path().join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    dir
}

/// A template a person wrote, with Blade logic around one LocDev block.
fn handwritten() -> String {
    let theme = default_theme();
    let block = emit(
        &[Segment::Block {
            id: "x".into(),
            block: "button".into(),
            props: json!({ "text": "Pay {{ $order->total }}", "url": "{{ $url }}" }),
            edited: false,
        }],
        &[],
        Flavor::Blade,
        &theme,
        false,
    );
    format!(
        "<p>Hi {{{{ $user->name }}}},</p>\n@if ($order->paid)\n    <p>Thanks!</p>\n@else\n{block}@endif\n{{{{-- a note --}}}}\n<footer>{{!! $footer !!}}</footer>"
    )
}

#[test]
fn round_trips_byte_for_byte() {
    for text in [
        starter(Flavor::Blade, "Welcome"),
        starter(Flavor::Markdown, "Welcome"),
        starter(Flavor::Html, "Welcome"),
        handwritten(),
        "no blocks at all\n@if($a) x @endif".into(),
    ] {
        let flavor = if text.contains("x-mail::message") {
            Flavor::Markdown
        } else if text.contains("<!-- locdev") {
            Flavor::Html
        } else {
            Flavor::Blade
        };
        let parsed = parse(&text, flavor, default_theme());
        assert_eq!(
            emit(
                &parsed.segments,
                &parsed.found,
                flavor,
                &parsed.theme,
                false
            ),
            text
        );
    }
}

#[test]
fn changing_a_block_leaves_blade_logic_untouched() {
    let text = handwritten();
    let mut parsed = parse(&text, Flavor::Blade, default_theme());
    for s in &mut parsed.segments {
        if let Segment::Block { props, .. } = s {
            props["text"] = json!("Pay now");
        }
    }
    let out = emit(
        &parsed.segments,
        &parsed.found,
        Flavor::Blade,
        &parsed.theme,
        false,
    );
    assert!(out.contains(">Pay now</a>"));
    for kept in [
        "<p>Hi {{ $user->name }},</p>\n@if ($order->paid)\n    <p>Thanks!</p>\n@else\n",
        "@endif\n{{-- a note --}}\n<footer>{!! $footer !!}</footer>",
        "href=\"{{ $url }}\"",
    ] {
        assert!(out.contains(kept), "lost {kept:?} in {out}");
    }
    assert_eq!(out.lines().count(), text.lines().count(), "{out}");
}

#[test]
fn markup_edited_outside_is_kept_and_flagged() {
    let text = handwritten().replace(
        "display:inline-block;padding:12px 22px",
        "display:block;padding:20px",
    );
    let parsed = parse(&text, Flavor::Blade, default_theme());
    assert!(matches!(
        &parsed
            .segments
            .iter()
            .find(|s| matches!(s, Segment::Block { .. }))
            .unwrap(),
        Segment::Block { edited: true, .. }
    ));
    // Moving things around (or a theme change) doesn't regenerate it.
    let out = emit(
        &parsed.segments,
        &parsed.found,
        Flavor::Blade,
        &parsed.theme,
        false,
    );
    assert!(out.contains("display:block;padding:20px"));
    // Clearing `edited` is the explicit "regenerate from properties".
    let regenerated: Vec<Segment> = parsed
        .segments
        .iter()
        .map(|s| match s {
            Segment::Block {
                id, block, props, ..
            } => Segment::Block {
                id: id.clone(),
                block: block.clone(),
                props: props.clone(),
                edited: false,
            },
            other => other.clone(),
        })
        .collect();
    let out = emit(
        &regenerated,
        &parsed.found,
        Flavor::Blade,
        &parsed.theme,
        false,
    );
    assert!(
        !out.contains("display:block;padding:20px")
            && out.contains("display:inline-block;padding:12px 22px")
    );
}

#[test]
fn text_escapes_html_but_keeps_blade_echoes() {
    let html = crate::blocks::text_html("5 < 6 & {{ $a <b> }} {!! $raw !!}\nline", Flavor::Blade);
    assert_eq!(html, "5 &lt; 6 &amp; {{ $a <b> }} {!! $raw !!}<br>line");
    assert_eq!(crate::blocks::text_html("{{ x }}", Flavor::Html), "{{ x }}");
    let tag = crate::blocks::component_tag(
        "x-mail.button",
        &json!({ "url": "{{ $url }}", "color": "red\"" }),
        "Go",
        Flavor::Blade,
    );
    assert_eq!(
        tag,
        "<x-mail.button color=\"red&quot;\" :url=\"$url\">Go</x-mail.button>"
    );
}

fn laravel() -> tempfile::TempDir {
    project(&[
        ("artisan", ""),
        ("resources/views/welcome.blade.php", "<h1>not mail</h1>"),
        (
            "resources/views/mail/invitation.blade.php",
            "{{ __('invite.subject', ['organization' => $organization]) }}\n\n{{ $url }}",
        ),
        (
            "resources/views/emails/order.blade.php",
            "<x-mail::message>\n# Order {{ $order->id }}\n<x-mail::button :url=\"$url\">View</x-mail::button>\n</x-mail::message>",
        ),
        (
            "app/Mail/InvitationMail.php",
            "<?php\nnamespace App\\Mail;\nclass InvitationMail extends Mailable {\n public function __construct(public Organization $organization, public string $token) {}\n public function content(): Content { return new Content(text: 'mail.invitation', with: [\n 'organization' => $this->organization->name,\n 'url' => route('x'),\n ]); }\n}",
        ),
        (
            "resources/views/components/mail/button.blade.php",
            "{{-- locdev:component {\"description\":\"A button\"} --}}\n@props(['url', 'color' => '#ea580c', 'size' => config('x')])\n<a href=\"{{ $url }}\" style=\"background:{{ $color }}\">{{ $slot }}</a>",
        ),
    ])
}

#[test]
fn discovers_laravel_templates_mailables_and_components() {
    let dir = laravel();
    let templates = crate::discover::templates(dir.path());
    let names: Vec<&str> = templates.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, vec!["emails.order", "mail.invitation"]);
    assert_eq!(templates[0].format, EmailFormat::Markdown);
    let invitation = &templates[1];
    assert_eq!(invitation.format, EmailFormat::Text);
    let m = &invitation.mailables[0];
    assert_eq!(m.class, "App\\Mail\\InvitationMail");
    assert_eq!(m.variables, vec!["organization", "token", "url"]);

    let components = crate::discover::components(dir.path());
    let button = components
        .iter()
        .find(|c| c.usage == "x-mail.button")
        .unwrap();
    let props: Vec<(&str, Option<&str>)> = button
        .props
        .iter()
        .map(|p| (p.name.as_str(), p.default.as_deref()))
        .collect();
    assert_eq!(
        props,
        vec![
            ("url", None),
            ("color", Some("'#ea580c'")),
            ("size", Some("config('x')"))
        ]
    );
    assert!(button.slot && button.description.as_deref() == Some("A button"));
    assert!(
        components
            .iter()
            .any(|c| c.usage == "x-mail::button" && c.builtin)
    );
    assert!(crate::resolve(dir.path(), "resources/views/welcome.blade.php").is_err());
}

#[test]
fn static_preview_substitutes_and_expands_without_running_anything() {
    let dir = laravel();
    let data = json!({ "user": { "name": "Ana <3" }, "url": "https://shop.test/pay", "order": { "id": 42, "paid": false } });
    let content = "<p>Hi {{ $user->name }}</p>\n@if ($order->paid)\n<b>paid</b>\n@endif\n<x-mail.button :url=\"$url\">Pay {{ $order->id }}</x-mail.button>\n{{ $missing ?? 'fallback' }} {{ strtoupper($x) }}";
    let out = crate::preview::render(content, EmailFormat::Blade, &data, dir.path());
    assert!(out.html.contains("<p>Hi Ana &lt;3</p>"));
    assert!(
        out.html
            .contains("<a href=\"https://shop.test/pay\" style=\"background:#ea580c\">Pay 42</a>"),
        "{}",
        out.html
    );
    assert!(out.html.contains("<b>paid</b>"), "both branches show");
    assert!(out.html.contains("fallback") && out.html.contains("‹strtoupper($x)›"));
    assert!(out.notes.iter().any(|n| n.contains("2 Blade directive")));

    let md = crate::preview::render(
        "<x-mail::message>\n# Order {{ $order->id }}\nSee **this**.\n<x-mail::button :url=\"$url\">View</x-mail::button>\n</x-mail::message>",
        EmailFormat::Markdown,
        &data,
        dir.path(),
    );
    assert!(
        md.html.contains("<h1")
            && md.html.contains("Order 42")
            && md.html.contains("<strong>this</strong>")
    );
    assert!(md.text.contains("View"));
}

#[test]
fn generator_follows_the_structure_and_never_overwrites() {
    let dir = laravel();
    let p = plan(dir.path(), Kind::Template, "order-shipped", false, true).unwrap();
    assert_eq!(
        p.artisan.as_deref(),
        Some(
            &[
                "make:mail".to_string(),
                "OrderShippedMail".into(),
                "--view=emails.order-shipped".into()
            ][..]
        )
    );
    let (view, content) = p.fill_after_artisan.unwrap();
    assert_eq!(view, "resources/views/emails/order-shipped.blade.php");
    assert!(content.contains("locdev:block") && content.contains("{{ config('app.name') }}"));

    let md = plan(dir.path(), Kind::Template, "receipt", true, false).unwrap();
    assert!(md.artisan.is_none() && md.files[0].1.contains("<x-mail::message>"));
    assert!(md.notes[0].contains("php artisan make:mail ReceiptMail --markdown=emails.receipt"));

    assert!(
        plan(dir.path(), Kind::Template, "order", false, true)
            .unwrap_err()
            .contains("already exists")
    );
    assert!(
        plan(dir.path(), Kind::Component, "button", false, true)
            .unwrap_err()
            .contains("already exists")
    );
    let footer = plan(dir.path(), Kind::Component, "footer", false, true).unwrap();
    assert_eq!(
        footer.files[0].0,
        "resources/views/components/mail/footer.blade.php"
    );
    assert!(
        crate::discover::blade_props(&footer.files[0].1)
            .iter()
            .any(|p| p.name == "company")
    );
    assert!(plan(dir.path(), Kind::Template, "../evil", false, true).is_err());

    let html = project(&[("package.json", "{}")]);
    let h = plan(html.path(), Kind::Template, "welcome", false, false).unwrap();
    assert_eq!(h.files[0].0, "emails/welcome.html");
}

#[test]
fn save_needs_the_reviewed_hash_and_keeps_history() {
    let dir = project(&[
        ("artisan", ""),
        ("resources/views/emails/a.blade.php", "<p>one</p>"),
    ]);
    let state = tempfile::tempdir().unwrap();
    let history = locdev_config::history::History::new(
        state.path(),
        "shop",
        "resources/views/emails/a.blade.php",
    );
    let doc = crate::open(dir.path(), "resources/views/emails/a.blade.php", json!({})).unwrap();
    let hash = crate::save(
        dir.path(),
        &doc.path,
        "<p>two</p>",
        &doc.hash,
        &history,
        "studio",
    )
    .unwrap();
    assert!(
        crate::save(
            dir.path(),
            &doc.path,
            "<p>three</p>",
            &doc.hash,
            &history,
            "studio"
        )
        .unwrap_err()
        .contains("changed since")
    );
    assert_ne!(hash, doc.hash);
    assert_eq!(history.list().len(), 1);
}
