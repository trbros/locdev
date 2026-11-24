# 監視フォルダー

監視フォルダーは、中のプロジェクトが自動でサイトになるフォルダーです。`~/Development` を監視すると、その中のプロジェクトそれぞれにドメインが付きます。

```text
~/Development/
├── shop/        → shop.test
├── api/         → api.test
├── portfolio/   → portfolio.test
├── dashboard/   → dashboard.test
└── photos/      (プロジェクトではないので除外)
```

一覧はフォルダーに追従します。そこで作成・クローン・生成されたプロジェクトは数秒で現れ、削除されたプロジェクトは一覧から外れます。ファイルには一切触れません。

## 対応状況

| システム | 状態                                                                           |
| -------- | ------------------------------------------------------------------------------ |
| Linux    | 利用可能 (テスト済み。inotify のイベント)                                      |
| macOS    | 実験的: FSEvents を使うコードですが、Mac では検証していません                  |
| Windows  | 実験的: ReadDirectoryChangesW を使うコードですが、Windows では検証していません |

どのシステムでも、ファイルイベントが使えない場合は 5 分ごとの再スキャンで動作します。

## プロジェクトとみなすもの

サブフォルダーは、ファイルからプロジェクトだと分かる場合だけサイトになります。判定は `locdev add` と同じ検出です ([フレームワーク](/docs/frameworks)): `artisan` と `composer.json` (Laravel)、`package.json` (Node.js、Next.js、Vite…)、`pyproject.toml`、`requirements.txt`、`manage.py` (Python)、`Cargo.toml` (Rust)、`index.php`、`index.html` など、フレームワーク表にあるものです。フォルダー名は関係ありません。

次のものはプロジェクトとみなさず、中も探しません。

- 隠しフォルダー (`.git`、`.cache`…)
- 依存関係とビルド出力: `node_modules`、`vendor`、`bower_components`、`target`、`dist`、`build`、`out`、`coverage`、`tmp`、`temp`、`cache`、`storage`、`venv`、`env`、`__pycache__`、`site-packages`、`Pods`、`DerivedData`
- 除外に指定したもの ([ルール](#ルール) を参照)

探索はプロジェクトの中には入りません。プロジェクトのサブフォルダー (モノレポのパッケージなど) が別のサイトになることはありません。シンボリックリンクはたどりますが、実体のフォルダーは一度しか訪れないので、上を指すリンクがあってもループや重複サイトにはなりません。

## フォルダーを監視する

アプリでは **設定 → 監視フォルダー → フォルダーを監視** を開き、フォルダーとルールを選びます。

![監視フォルダー](screenshot:parked-folders)

CLI では次のとおりです (パスを省くとカレントフォルダー)。

```bash
locdev park
locdev park ~/Development
locdev park ~/Clients --depth 2 --exclude 'archive-*'
```

次のフォルダーは監視できません。

- それ自体がプロジェクトのフォルダー (`locdev add` で追加します)
- 監視フォルダーの中にあるフォルダー、または監視フォルダーを含むフォルダー (各プロジェクトは一つの経路からしか見つけられません)
- ドライブのルート

## ルール

![フォルダーを監視](screenshot:park-folder)

| ルール                                   | 選択肢                                                              | デフォルト   |
| ---------------------------------------- | ------------------------------------------------------------------- | ------------ |
| プロジェクトを探す範囲                   | 直下のフォルダー (1)、2 階層まで、3 階層まで                        | 1            |
| 新しいプロジェクトが現れたとき           | **サイトにする** または **提案する** (一覧に出て、自分で追加)       | サイトにする |
| プロジェクトのフォルダーが削除されたとき | **サイトを外す** または **残して「見つからない」と表示**            | サイトを外す |
| 除外するフォルダー                       | 名前または相対パス。`*` は任意の文字列 (`archive-*`、`clients/old`) | なし         |

ルールを変えるには、同じフォルダーでもう一度 `locdev park` を実行します。指定したものだけが変わります。

```bash
locdev park ~/Development --suggest          # 新しいプロジェクトは提案だけ
locdev park ~/Development --keep-missing     # 削除されたフォルダーのサイトを残す
locdev park ~/Development --no-exclude       # 除外リストを空にする
```

深さ 2 や 3 では、プロジェクトではないフォルダーだけをたどって下ります。`~/Clients/acme/shop` の `shop` が見つかるのは、`acme` がプロジェクトでない場合だけです。

## 見つかったプロジェクト

各プロジェクトは、フォルダーの一覧に状態付きで表示されます。

| 状態 (アプリ) | CLI                    | 意味                                                           |
| ------------- | ---------------------- | -------------------------------------------------------------- |
| サイト        | `site`                 | このフォルダーによってサイトになった                           |
| 手動で追加    | `site (added by hand)` | もともと自分で追加したサイト。そのまま残る                     |
| 提案          | `suggested`            | 追加待ち (アプリの **追加**、または `locdev add <フォルダー>`) |
| 無視          | `ignored`              | このプロジェクトをサイトにしない                               |
| 未追加        | `not added`            | サイトを作れなかった (理由が横に表示される)                    |

```bash
locdev park list
```

```text
/home/you/Development  watching · depth 1 · new: become sites · gone: removed · scanned 12s ago
  api        api.test                      Rust                site
  dashboard  dashboard.test                Next.js             site
  portfolio  portfolio.test                HTML5               site
  shop       shop.test                     Laravel             site
```

新しいサイトは **停止状態** で、自動起動もオフです。頼むまで何も動きません。ポート、コマンド、ランタイムは `locdev add` と同じく検出から決まります。

### 名前の重複

サイト名はフォルダー名です。すでに使われている場合は、一つ上のフォルダー名を付けます (`~/Clients/shop` なら `shop-clients`、`~/Development/shop` なら `shop-development`)。それも使われていれば番号を付けます (`shop-2` から `shop-9`)。理由はプロジェクトの横に表示されます。一度登録した名前は変わらず、以降のスキャンでサイト名が変わることはありません。

### プロジェクトを無視する

```bash
locdev park ignore ~/Development/scratch
locdev park unignore ~/Development/scratch
```

無視すると、(そのフォルダーから追加された) サイトを外し、戻ってこないようにします。監視フォルダーから追加されたサイトをアプリや `locdev remove` で外した場合も同じで、そのフォルダーでは無視扱いになります。そうしないと次のスキャンで戻ってくるためです。

## スキャンとイベント

LocDev がフォルダーをスキャンするのは、監視を始めたときやルールを変えたとき、中で何かが変わったとき (ファイルイベント。フォルダーが 1.5 秒静かになるまでまとめるので、`git clone` や `laravel new` は 1 回の変更として扱われます)、そして取りこぼしを補うための 5 分ごとです。手動で実行するには次のとおりです。

```bash
locdev park scan
locdev park scan ~/Development
```

イベントを受け取るのは、監視フォルダーとその間にある (プロジェクトではない) フォルダーだけです。プロジェクトの中は監視しないので、`node_modules` でシステムの監視数の上限を使い切ることはありません。

監視フォルダー自体が消えた場合 (外付けドライブを外した、フォルダー名を変えたなど) は、サイトを一つも外しません。フォルダーは「フォルダーが見つかりません」と表示され、戻ればサイトも元どおり動きます。

## 一時停止と監視の終了

```bash
locdev park disable ~/Development    # スキャンもイベントもなし。サイトは残る
locdev park enable ~/Development
locdev park remove ~/Development     # このフォルダーから追加されたサイトを LocDev から外す
locdev park remove ~/Development --keep-sites
```

`park remove` が停止して外すのは、そのフォルダーから追加されたサイトだけです。手動で追加したサイトは残ります。`--keep-sites` を付けると、サイトは通常のサイトとして残ります。どの場合もフォルダーやファイルは削除しません。

## 保存場所

フォルダーとルールは、LocDev の設定フォルダーの `config/parked.toml` にあります ([設定](/docs/configuration))。監視フォルダーから追加されたサイトは、`projects.toml` の `parked` に元のフォルダーを保持します。

## 既知の問題

- **"is itself a project":** プロジェクトのフォルダーを監視しようとしました。`locdev add` で追加するか、一つ上のフォルダーを監視してください。
- **"overlaps the parked folder":** 監視フォルダーの中にある (または監視フォルダーを含む) フォルダーです。代わりに外側の深さを上げてください。
- **「5 分ごとに再スキャン」/ "watching unavailable":** システムがファイルイベントを拒否しました (Linux では多くの場合 `fs.inotify.max_user_watches` の上限)。動作は続きますが、反映が最大 5 分遅れます。
- **プロジェクトが表示されない:** 検出が認識するファイルがあるか、除外フォルダーの中にないか、深さが届いているか、無視になっていないかを確認してください。
