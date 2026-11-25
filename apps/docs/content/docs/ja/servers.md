# 開発サーバー

LocDev は各サイトの開発サーバーを、デーモンが監視するプロセスとして動かします。正しい Node.js/PHP のバージョンで起動し、ポートを割り当てられ、`https://<site>.test` でプロキシの背後に置かれ、アプリのウィンドウを閉じても動き続けます。

## 検出

フォルダーを登録すると、LocDev はプロジェクトのファイル (`package.json`、ロックファイル、`vite.config.*`、`artisan`、`composer.json`、`wp-config.php` など) を**読んで**コマンドを選びます。技術を判別するためだけにプロジェクトのスクリプトを実行することはありません。

| プロジェクト                       | メインサーバー                                                                                       | ポートの渡し方                                |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| Laravel                            | `php artisan serve --no-reload` (`--no-reload` がないと、LocDev が渡す変数を Laravel が捨ててしまう) | `--port {port}`                               |
| Laravel + Vite                     | `artisan serve` + **補助としての Vite** (同意が必要)                                                 | Vite に `--port`                              |
| React/Vue/Svelte + Vite、SvelteKit | `dev` スクリプト                                                                                     | `--port {port} --strictPort --host 127.0.0.1` |
| Next.js                            | `dev` スクリプト                                                                                     | `--hostname localhost --port {port}`          |
| Nuxt、Astro                        | `dev` スクリプト                                                                                     | `--port {port} --host 127.0.0.1`              |
| 汎用 Node.js                       | `dev` または `start`                                                                                 | `PORT` 変数                                   |
| PHP、WordPress                     | `php -S`                                                                                             | コマンドラインで指定                          |
| HTML/CSS/JS                        | プロセスなし                                                                                         | —                                             |

Vite は `PORT` 変数を無視するので、LocDev は `--port` と `--strictPort` を渡します (ポートが使用中なら、Vite はプロキシが知らない別のポートに移らずに失敗します)。パッケージマネージャーはロックファイル (`pnpm-lock.yaml`、`yarn.lock`、`bun.lock`、なければ npm) で決まります。`locdev info` で検出結果を確認でき、登録済みのコマンドが LocDev が今使うものと違う場合はサイトの詳細に警告が出ます。

## 補助サーバー (Laravel と並ぶ Vite)

Vite を使う Laravel プロジェクトでは、PHP がページを、Vite がホットリロード付きでアセットを提供します。つまりプロセスは 2 つです。LocDev は Vite を検出しますが、**あなたが決めるまで何も実行しません**。

![検出された Vite についての確認](screenshot:dev-prompt)

| 選択肢             | 効果                                                     |
| ------------------ | -------------------------------------------------------- |
| 今すぐ起動         | 今 Vite を実行します。次回からは好きなときに起動します。 |
| サイトと一緒に起動 | サイトと一緒に起動・停止します。                         |
| また聞く           | 何も実行しません。確認は残ります。                       |
| 管理しない         | LocDev は確認をやめ、Vite を実行しません。               |

この選択は、サイトの **サーバー** タブまたは CLI でいつでも変更できます。

```bash
locdev dev mode auto            # サイトと一緒に起動
locdev dev mode manual --start  # 今起動し、以降は手動
locdev dev mode never
locdev dev status | start | stop | restart
locdev dev logs -f
```

補助サーバーは専用のポートと出力を持ちます (`locdev logs --dev`)。サイトを停止すると両方が止まります。Vite は SIGTERM (Windows ではツリーの終了) を受け取るので、`laravel-vite-plugin` が `public/hot` ファイルを自分で削除します。

## プロキシ、HMR、HTTPS

- **許可するホスト:** Vite は未知のホストを拒否します (`.localhost` は常に許可)。LocDev は `__VITE_ADDITIONAL_SERVER_ALLOWED_HOSTS=<site>.test,.<site>.test` 変数をプロセスに渡し、Vite がそれを読みます。プロジェクトの `vite.config` は編集せず、ほかのホストも許可しません。
- **HMR:** Vite のクライアントはページと同じホストとプロトコルに接続するので、WebSocket はプロキシを通ります (`ws://` または `wss://`)。Vite 8 で確認済みです。ファイルを編集すると、`101` ハンドシェイクと `full-reload` がプロキシ経由で届きます。
- **HTTPS での Laravel + Vite:** ページは `https://<site>.test` から、アセットは Vite から届きます。LocDev の CA が信頼され、`*.test` が LocDev に解決される場合、Vite のプロセスには `VITE_DEV_SERVER_CERT`、`VITE_DEV_SERVER_KEY` (ローカル CA が発行したサイトの証明書。鍵はあなたのユーザーだけが読める) と `APP_URL=https://<site>.test` が渡されます。すると `laravel-vite-plugin` がアセットを HTTPS で提供し、_mixed content_ になりません。証明書は `localhost`、`127.0.0.1`、`::1` もカバーします。Vite 8.3 以降では、Vite に `--host 127.0.0.1` を渡すとプラグインが設定するサイトのホストが失われ、プラグインは `public/hot` に `https://127.0.0.1:<port>` を書き込み、ページはそのアドレスからアセットを読み込むためです。条件を満たさない場合、Vite は `http://127.0.0.1:<port>` のままですが、ループバックなので Chrome と Firefox は HTTPS のページでも受け付けます。
- プラグインは `*.test` のオリジンに対して CORS も許可します。

どれもプロジェクトのファイルは変更しません。すべてプロセスの環境にだけ渡されます。

> [!WARNING]
> Next.js 15 以降では、別のオリジンからの開発リクエストについて警告が出ることがあります。`<site>.test` 経由でホットリロードが動かない場合は、`next.config` の `allowedDevOrigins` にドメインを追加してください。LocDev がこのファイルを変更することはありません。

## 監視

- **インスタンスは 1 つ:** 動作中のサイトを起動しても、別のプロセスは立ち上がりません。
- **ポート使用中:** 起動前に LocDev がポートを確認し、使用中なら誰が使っているか (プロセスと PID) を表示します。
- **プロセスツリー:** 各サイトは専用のプロセスグループ (Unix) またはツリー (Windows) で動きます。停止すると、コマンドが作ったものすべて (npm → node、artisan → php) が止まります。8 秒たっても終了しなければ強制終了します。
- **失敗時の再起動:** サイトごとに任意 (サイトの Settings、または `locdev edit <site> --restart-on-failure true`)。待ち時間は徐々に延び (1 秒、2 秒、4 秒…)、2 分間で最大 5 回まで試します。
- **デーモンが落ちたあと:** LocDev は実行中のプロセスを記録しています。新しいデーモンは前のデーモンが残したものを終了し (出力はつなぎ直せないため)、自動起動の設定があるものを起動し直します。

## プロセスとログ

![ツール → プロセスとログ](screenshot:processes)

**Tools → Processes & logs** (または `locdev ps`) には、すべてのプロセスが PID、ポート、ランタイム、稼働時間、状態、再起動回数、CPU とメモリ (プロセスツリーの合計) とともに表示されます。コンソールには stdout、stderr、LocDev のメモが表示され、ストリームとレベルでの絞り込み、検索、一時停止、自動スクロール、コピーができます。

レベル (`error`、`warn`、`info`、`debug`) は、行自体が宣言している場合にだけ表示されます: `ERROR:`、`[warn]`、`npm ERR!`、`PHP Fatal error:`、Laravel の `local.ERROR:`。通常の出力にはレベルがなく、stderr だからといってエラー扱いにはしません。

出力はメモリ (プロセスごとに 2000 行) とディスク (`logs/sites/<site>.log`。5 MB でローテーションし、1 つ前のファイルを保持) に保存されます。そのため、プロセスが終了したあとも、デーモンを再起動したあとも見られます。上限は `config.toml` で設定します。

```toml
[logs]
lines = 2000
file_mb = 5
```

これはプロセスの出力による診断です。対話的なデバッグ (Xdebug、Node のインスペクター) は今のところ LocDev には含まれていません。

## 補助プロセス

開発サーバーと補助の Vite のほかに、サイトは**補助プロセス**を持てます。キューワーカー、スケジューラー、WebSocket サーバー、独自のサーバーを持つツールです。フレームワークのアダプターは、プロジェクトにインストールされているものを提案します。

| スタック    | 提案されるプロセス                                                               | 条件                                                   |
| ----------- | -------------------------------------------------------------------------------- | ------------------------------------------------------ |
| Laravel     | Queue worker (`php artisan queue:work`)、Scheduler (`php artisan schedule:work`) | 常に                                                   |
| Laravel     | Horizon (`php artisan horizon`)                                                  | `composer.json` に `laravel/horizon`                   |
| Laravel     | Reverb (`php artisan reverb:start`)                                              | `composer.json` に `laravel/reverb`                    |
| Rails       | Solid Queue (`bin/jobs`)                                                         | `solid_queue` gem と `bin/jobs`                        |
| Rails       | Sidekiq (`bundle exec sidekiq`)                                                  | `sidekiq` gem                                          |
| Django      | Celery worker と Celery beat (`celery -A <project> …`)                           | 依存関係に `celery`。プロジェクト名は `manage.py` から |
| すべての JS | Storybook (`<pm> run storybook --port <port> …`)                                 | `storybook` と `storybook` スクリプト                  |

補助の Vite と同じく、**決めるまで何も実行されません**。提案された各プロセスは _ask_ から始まります。モードは _手動_ (自分で起動)、_サイトと一緒_ (サイトと一緒に起動)、_なし_ です。

同じジョブを処理してしまう選択肢 (Horizon と `queue:work`、Solid Queue と Sidekiq) は同時には動きません。一方が動いているときにもう一方を起動しようとすると拒否され、片方を _with site_ にするともう片方はそのモードから外れます。ワーカーとスケジューラーは落ちると自動で再起動します (サーバーと同じバックオフ)。各プロセスは専用のログを持ち、サイトのランタイムと共有変数を受け取り、サイトと一緒に止まります。

自分のプロセス (別のキューのワーカー、ウォッチャーなど) も追加できます。コマンド中の `{port}` には空きポートが入ります。

```bash
locdev process list
locdev process mode queue auto
locdev process start scheduler
locdev process mode queue manual --command "php artisan queue:work --queue=high,default"
locdev process add mailer --command "php artisan queue:work --queue=mail" --label "Mail queue"
locdev process logs queue -f
```

アプリでは、サイトの **サーバー** タブに、各プロセスのモード、操作、コンソールが一覧表示されます。**Tools → Processes & logs** には動作中のものが表示されます。
