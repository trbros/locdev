//! The framework renders the template (Laravel): a PHP script LocDev owns
//! boots the app and renders the view, run by the daemon as a separate,
//! time-limited process with a reduced environment (see `daemon/email.rs`).
//! The unsaved editor content goes into a view file under LocDev's own temp
//! folder, never into the project.

/// The script: `php render.php <root> <views-dir> <view> <markdown|html>`,
/// sample data as JSON on stdin (objects become stdClass, so `$user->name` works).
pub const LARAVEL_RENDER: &str = r#"<?php
// Written by LocDev: renders one view for the Email Studio preview.
[$script, $root, $dir, $view, $mode] = $argv;
chdir($root);
require $root.'/vendor/autoload.php';
$app = require $root.'/bootstrap/app.php';
$app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap();
app('view')->addLocation($dir);
$data = json_decode(stream_get_contents(STDIN));
$data = is_object($data) ? get_object_vars($data) : [];
if ($mode === 'markdown') {
    echo app(Illuminate\Mail\Markdown::class)->render($view, $data);
} else {
    echo view($view, $data)->render();
}
"#;

/// Laravel can render here: Composer's autoloader and the app's bootstrap exist.
pub fn laravel_ready(root: &std::path::Path) -> bool {
    root.join("vendor/autoload.php").is_file() && root.join("bootstrap/app.php").is_file()
}
