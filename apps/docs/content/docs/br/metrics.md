# Métricas de requisições

Toda requisição a um site pelo proxy do LocDev (`<site>.test`, seus subdomínios, HTTP e HTTPS) entra na conta. A aba **Métricas** do site (ou `locdev metrics <site>`) mostra, atualizada a cada 3 segundos:

- requisições e erros 5xx;
- latência (média, p50, p95, p99 e a mais lenta) como o tempo até os cabeçalhos da resposta, nas últimas 1.000 requisições;
- requisições por minuto na última hora;
- códigos de status por classe, mais as requisições que o servidor do site não respondeu;
- os caminhos mais acessados (método e caminho, sem a query) com tempo médio e erros;
- as últimas 100 requisições.

## O que conta e o que não conta

Os números ficam só na memória do daemon: nunca vão para o disco nem para lugar nenhum. Reiniciar o daemon, ou **Zerar** (`locdev metrics <site> --reset`), começa do zero. Requisições direto para `127.0.0.1:<porta>` não passam pelo proxy e não entram na conta. Um upgrade de WebSocket conta como uma requisição (101).

A memória é limitada por site: 1.000 latências, 60 minutos, 100 amostras e 300 caminhos diferentes (caminhos além disso ainda entram nos totais).

```bash
locdev metrics
locdev metrics loja --json
locdev metrics loja --reset
```

Respostas 5xx e servidor inacessível também aparecem no [Debug Center](/docs/debug-center).
