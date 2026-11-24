# コードエディター

LocDev は、サイトやそのファイルを指定した行でコードエディターに開けます。サイトのメニュー(選んだエディター名で **VS Code で開く**)、サイトページでの `Ctrl` `E`、**Config** と **Email Studio** タブのボタン、または `locdev editor open` から使えます。

![設定 → 連携: 見つかったエディターと選択中のエディター。](screenshot:editor-settings)

## エディター

LocDev は各エディターのコマンドラインランチャーを、`PATH`、インストーラーが使うフォルダー(`/usr/local/bin`、Homebrew、`~/.local/bin`、`/snap/bin`、JetBrains Toolbox のスクリプト)、一般的なインストール先から探します。

| エディター                                            | コマンド                                                 | 行を指定して開く               |
| ----------------------------------------------------- | -------------------------------------------------------- | ------------------------------ |
| Visual Studio Code                                    | `code`                                                   | `code --goto ファイル:行:列`   |
| Cursor                                                | `cursor`                                                 | VS Code と同じ                 |
| Windsurf                                              | `windsurf`                                               | VS Code と同じ                 |
| VSCodium                                              | `codium`                                                 | VS Code と同じ                 |
| Zed                                                   | `zed`(一部の Linux ディストリビューションでは `zeditor`) | `zed ファイル:行:列`           |
| Sublime Text                                          | `subl`                                                   | `subl ファイル:行:列`          |
| PhpStorm、WebStorm、IntelliJ IDEA、PyCharm、RustRover | `phpstorm`、`webstorm`、`idea`、`pycharm`、`rustrover`   | `--line N --column M ファイル` |

出典: [VS Code のコマンドライン](https://code.visualstudio.com/docs/configure/command-line)、[Zed の CLI](https://zed.dev/docs/reference/cli)、[Sublime Text のコマンドライン](https://www.sublimetext.com/docs/command_line.html)、[JetBrains IDE でコマンドラインからファイルを開く](https://www.jetbrains.com/help/idea/opening-files-from-command-line.html)。

エディターが表示されない場合は、ターミナルでそのコマンドが使えるか確認してください。Linux と Windows では VS Code とその派生版のインストーラーが `PATH` に追加します。macOS ではエディターのコマンドパレット → **Shell Command: Install 'code' command in PATH** を使います。JetBrains IDE のスクリプトは JetBrains Toolbox が作成します(設定でシェルスクリプトの生成を有効にします)。その後 **もう一度探す**(または `locdev editor list --refresh`)。

## エディターを選ぶ

**設定 → 連携 → コードエディター** でエディターをクリックします。選んでいない場合は、表の順で最初に見つかったものが開きます。選択は `settings.toml` の `[editor] preferred` に保存されます。選んだエディターをアンインストールすると、LocDev は最初に見つかったものを使い、その旨を表示します。

```bash
locdev editor list
locdev editor set zed
locdev editor open shop
locdev editor open shop app/Http/Kernel.php:42
```

`locdev editor set none` で最初に見つかったものに戻り、`--editor cursor` でその回だけ別のエディターで開きます。

## 安全性

- サイトのフォルダーか、その中のファイルだけ: 相対パスで、`..` やフォルダーの外を指すリンクは使えません(デーモンが実際のパスを確認します)。
- エディターはシェルを使わず、各エディターが文書化している引数で起動し、独立して動作します。LocDev は終了を待ちません。
- 表にあるエディターだけです。カスタムコマンドはありません。

## 制限事項

- デーモンは自身の環境でエディターを起動します。ログイン時に起動したデーモン(環境にグラフィカルセッションがない場合)では、GUI のエディターが開かないことがあります。アプリからデーモンを再起動してください。
- ターミナルで動くエディター(Vim、Neovim、Helix、ターミナルの Emacs)は、ターミナルが必要なため対象外です。
- Linux で代わりのエディターと snap 版 VS Code の検出でテストしました。Windows と macOS では実機テストしていません。
