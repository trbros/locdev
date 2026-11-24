//! Payloads captured from symfony/var-dumper 7.4.22 in `server` format
//! (`VAR_DUMPER_FORMAT=server`, a socket recording each line), from:
//!
//! ```php
//! dump(['id' => 7, 'ok' => true, 'ratio' => 0.25, 'none' => null, 'list' => [1, 2, [3]]]);
//! $a = new User(); $b = new User(); $a->link($b); $b->link($a); $a->extra = 'dyn';
//! dump($a);                         // private/protected/public/dynamic, recursion
//! dump(str_repeat('x', 5000));
//! dump(Suit::Hearts);               // backed enum
//! dump("plain string");
//! dump(fn (int $x) => $x * 2);
//! dump(new DateTimeImmutable('2026-11-05 10:00:00', new DateTimeZone('UTC')));
//! dump(['token' => 'abc', 'nested' => ['api_key' => 'k-123456']], 42);
//! ```

use locdev_core::observe::{DumpNode, KeyKind};
use locdev_observe::dump::{decode, summary};

fn fixtures() -> Vec<String> {
    include_str!("fixtures/vardumper-7.4.txt")
        .lines()
        .map(str::to_string)
        .collect()
}

fn items(node: &DumpNode) -> &[locdev_core::observe::DumpEntry] {
    match node {
        DumpNode::Array { items, .. } | DumpNode::Object { items, .. } => items,
        other => panic!("no items in {other:?}"),
    }
}

#[test]
fn arrays_and_scalars() {
    let d = decode(&fixtures()[0]).unwrap();
    assert_eq!(summary(&d.value), "array:5");
    let e = items(&d.value);
    assert_eq!(e[0].key, "id");
    assert_eq!(e[0].value, DumpNode::Int { v: 7 });
    assert_eq!(e[1].value, DumpNode::Bool { v: true });
    assert_eq!(e[2].value, DumpNode::Float { v: 0.25 });
    assert_eq!(e[3].value, DumpNode::Null);
    let list = items(&e[4].value);
    assert_eq!(list.len(), 3);
    assert_eq!(summary(&list[2].value), "array:1");
    assert_eq!(d.file.as_deref(), Some("/tmp/locdev-fixture/app.php"));
    assert_eq!(d.line, Some(14));
    assert!(d.command.is_some(), "a CLI dump carries its command line");
    assert!(d.timestamp.unwrap() > 1.7e9);
}

#[test]
fn objects_keep_visibility_and_stop_at_recursion() {
    let d = decode(&fixtures()[1]).unwrap();
    let DumpNode::Object {
        class,
        handle,
        items,
        ..
    } = &d.value
    else {
        panic!("{:?}", d.value)
    };
    assert_eq!(class, "User");
    let kinds: Vec<_> = items.iter().map(|e| (e.key.as_str(), e.kind)).collect();
    assert!(kinds.contains(&("secret", KeyKind::Private)));
    assert!(kinds.contains(&("prot", KeyKind::Protected)));
    assert!(kinds.contains(&("name", KeyKind::Public)));
    assert!(kinds.contains(&("extra", KeyKind::Dynamic)));
    let friend = &items.iter().find(|e| e.key == "friend").unwrap().value;
    let DumpNode::Object { items: inner, .. } = friend else {
        panic!("{friend:?}")
    };
    let back = &inner.iter().find(|e| e.key == "friend").unwrap().value;
    assert_eq!(
        back,
        &DumpNode::Seen {
            handle: *handle,
            class: Some("User".into())
        },
        "the r: back-reference points to the first User"
    );
}

#[test]
fn strings_enums_closures_and_dates() {
    let all = fixtures();
    let DumpNode::Str { v, .. } = decode(&all[2]).unwrap().value else {
        panic!()
    };
    assert_eq!(v.len(), 5000);
    let suit = decode(&all[3]).unwrap().value;
    assert_eq!(summary(&suit).split(' ').next(), Some("Suit"));
    assert_eq!(
        items(&suit)[0].value,
        DumpNode::Str {
            v: "Hearts".into(),
            cut: 0,
            binary: false
        }
    );
    assert_eq!(
        decode(&all[4]).unwrap().value,
        DumpNode::Str {
            v: "plain string".into(),
            cut: 0,
            binary: false
        }
    );
    assert!(summary(&decode(&all[5]).unwrap().value).starts_with("Closure(int $x)"));
    let date = decode(&all[6]).unwrap().value;
    let meta = &items(&date)[0];
    assert_eq!((meta.key.as_str(), meta.kind), ("date", KeyKind::Meta));
    assert!(
        matches!(&meta.value, DumpNode::Text { v } if v.starts_with("2026-11-05 10:00:00.0 UTC"))
    );
}

#[test]
fn two_arguments_are_two_dumps() {
    let all = fixtures();
    assert_eq!(all.len(), 9);
    assert_eq!(decode(&all[8]).unwrap().value, DumpNode::Int { v: 42 });
}

#[test]
fn garbage_is_refused() {
    assert!(decode("not base64 at all!").is_err());
    assert!(decode("YTowOnt9").is_err(), "a:0:{{}} has no data");
}
