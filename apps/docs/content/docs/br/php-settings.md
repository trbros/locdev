# Configurações do PHP

Em **Runtimes → PHP → configurações** (ou `locdev php`), você ajusta `memory_limit`, o tamanho de upload, o fuso horário e as outras diretivas de cada versão do PHP, sem editar o `php.ini` do sistema.

![Configurações do PHP: o valor do próprio PHP, o do LocDev e o que está em uso.](screenshot:php-settings)

## Como funciona

Os sites do LocDev rodam no **servidor embutido do PHP** (`php -S`, `artisan serve`), que lê os mesmos arquivos que o comando `php`: o `php.ini` carregado e os arquivos da pasta de scan (`conf.d`). O LocDev não muda nenhum deles. Ele mantém **um arquivo próprio por versão** (`config/php/<versão>/90-locdev.ini`, com a marca "Managed by LocDev") e o acrescenta aos processos que ele roda pelo `PHP_INI_SCAN_DIR`, depois da pasta de scan do PHP. Por isso o valor do LocDev vence, e só para os sites do LocDev: o `php` do seu terminal continua como estava.

Para cada diretiva a tela mostra três valores:

| Coluna     | De onde vem                                                                       |
| ---------- | --------------------------------------------------------------------------------- |
| **LocDev** | o arquivo do LocDev (vazio: não mexe)                                             |
| **Do PHP** | o que o PHP lê sem o LocDev: `php.ini`, pasta de scan ou o padrão do próprio PHP  |
| **Em uso** | o que o PHP realmente carrega com o arquivo do LocDev (em destaque quando difere) |

Nada tem valor padrão inventado: enquanto você não muda uma diretiva, vale a do seu PHP.

As versões são agrupadas por linha (`8.3`, `8.4`): as configurações de 8.3 valem para qualquer 8.3.x que os sites usem. A tela mostra qual instalação foi lida, o `php.ini` carregado e os arquivos da pasta de scan, que você pode abrir **somente para leitura**.

## Diretivas

| Diretiva                 | Tipo                                           |
| ------------------------ | ---------------------------------------------- |
| `memory_limit`           | tamanho (`512M`, `2G`, `-1` = sem limite)      |
| `upload_max_filesize`    | tamanho                                        |
| `post_max_size`          | tamanho                                        |
| `max_file_uploads`       | número                                         |
| `max_execution_time`     | segundos (`0` = sem limite)                    |
| `max_input_time`         | segundos (`-1` = usa o limite de execução)     |
| `max_input_vars`         | número                                         |
| `display_errors`         | On/Off                                         |
| `display_startup_errors` | On/Off                                         |
| `log_errors`             | On/Off                                         |
| `error_reporting`        | constantes (`E_ALL & ~E_DEPRECATED`) ou número |
| `date.timezone`          | fuso (`America/Sao_Paulo`, `UTC`)              |
| `default_socket_timeout` | segundos                                       |
| `realpath_cache_size`    | tamanho                                        |
| `opcache.enable_cli`     | On/Off (o OPcache no servidor embutido)        |

Valores fora do formato são recusados antes de qualquer coisa ser escrita. A tela avisa combinações que não funcionam juntas (`upload_max_filesize` maior que `post_max_size`, `post_max_size` maior que `memory_limit`) e um fuso que o PHP não conhece.

> [!NOTE]
> A linha de comando do PHP força `max_execution_time = 0` e `max_input_time = -1`, mas o servidor embutido, que serve os sites, usa o valor dos arquivos. A tela mostra o valor que os sites recebem. Comandos (`artisan`, `composer`) continuam sem limite de tempo.

## Aplicar uma mudança

1. Mude os valores na coluna **LocDev** (apagar o campo volta ao valor do PHP).
2. **Revisar mudanças** mostra as linhas que entram e saem do arquivo do LocDev e quais sites em execução vão reiniciar.
3. Em **Aplicar**, o LocDev guarda o arquivo atual no histórico, escreve o novo e **confere com o próprio PHP** o que ele carrega. Se o PHP recusar (erro de sintaxe, valor ignorado), o arquivo anterior volta e nada mais muda.
4. Só os sites rodando com aquela versão do PHP reiniciam, cada um com todos os seus processos (servidor, Vite, filas). O app e o daemon continuam abertos.

**Usar os valores do PHP** apaga o arquivo do LocDev daquela versão (também passa pela revisão).

## Configuração avançada

**Configuração avançada** abre o arquivo do LocDev num editor, com realce e `Ctrl+S`. Pode entrar qualquer diretiva que o PHP aceite (`xdebug.mode`, `opcache.*`…), e linhas e comentários que você escreveu são mantidos quando você muda algo pelo formulário. Antes de aplicar, o arquivo é conferido pelo PHP numa pasta temporária: um erro de sintaxe é mostrado e nada é gravado. Se o arquivo mudou desde que você o abriu (por outra janela ou pela CLI), salvar é recusado para você recarregar.

O `php.ini` do sistema e os arquivos da pasta de scan aparecem **somente para leitura**: para mudar um valor deles, sobrescreva no arquivo do LocDev.

## Histórico

Cada mudança guarda o arquivo que ela substituiu (os últimos 30 por versão, em `data/state/php-ini-history/<versão>/`). **Histórico → Restaurar…** volta a uma versão anterior, com a mesma revisão e conferência.

## Na linha de comando

```bash
locdev php lines                                   # versões e se o LocDev configura alguma
locdev php show                                    # a versão padrão do LocDev
locdev php show --line 8.3
locdev php set memory_limit=512M upload_max_filesize=64M post_max_size=64M
locdev php set date.timezone=America/Sao_Paulo --line 8.3 --yes
locdev php unset memory_limit
locdev php reset --line 8.3
locdev php edit                                    # o arquivo do LocDev no $EDITOR
locdev php file /etc/php/8.3/cli/php.ini           # um arquivo que o PHP carrega, só leitura
locdev php history
locdev php restore 1793830885840
```

`set`, `unset`, `reset`, `edit` e `restore` mostram a mudança e perguntam antes (`--yes` pula a pergunta).

## Limites

- Vale para os processos que o LocDev roda (sites, servidores auxiliares, filas). O `php` que você roda no terminal não recebe o arquivo do LocDev.
- PHP-FPM e Apache/Nginx não são usados pelo LocDev; a diferença entre a CLI e o servidor web está só nos dois limites de tempo acima.
- Validado no Linux com o PHP do sistema e do Yerd (um site real servido pelo servidor embutido recebendo os valores novos depois do reinício). Windows e macOS não foram testados; no Windows o separador do `PHP_INI_SCAN_DIR` é `;`.
