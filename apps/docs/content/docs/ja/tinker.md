# Tinker

Laravel サイトには **Tinker** タブがあり、サイトの PHP バージョン、環境 (共有変数、メールとダンプの設定)、`.env` を使って、サーバーとまったく同じようにアプリ内で PHP を実行できます。

- **実行** (`Ctrl`/`⌘` `Enter`) はエディターのコードを `php artisan tinker --execute` で実行します。実行ごとに新しく始まり、最大 30 秒、出力は 256 KB までです。
- **コンソール** は対話型 REPL (`php artisan tinker`) をアプリ内のターミナルで開きます。終了するまで行の間で変数が保たれます。

`laravel/tinker` (Laravel のスケルトンに含まれます) とインストール済みの依存関係 (`composer install`) が必要です。コードはあなたが求めたときだけ実行され、あなたのアプリとしてサイトのデータベースやサービスにアクセスします。

## ターミナルで

```bash
locdev tinker -e 'App\Models\User::count()'
echo 'config("app.name")' | locdev tinker
locdev tinker --site shop      # プロンプト。各行は個別に実行されます
```
