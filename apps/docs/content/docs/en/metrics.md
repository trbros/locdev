# Request metrics

Every request to a site through LocDev's proxy (`<site>.test`, its subdomains, HTTP and HTTPS) is counted. The site's **Metrics** tab (or `locdev metrics <site>`) shows, updated every 3 seconds:

- requests and 5xx errors;
- latency (average, p50, p95, p99 and the slowest) as the time to the response headers, over the last 1,000 requests;
- requests per minute in the last hour;
- status codes by class, plus requests the site's server didn't answer;
- the busiest paths (method and path, without the query) with their average time and errors;
- the last 100 requests.

## What is and isn't counted

The numbers live only in the daemon's memory: they're never written to disk or sent anywhere. Restarting the daemon, or **Reset** (`locdev metrics <site> --reset`), starts over. Requests straight to `127.0.0.1:<port>` skip the proxy and aren't counted. A WebSocket upgrade counts as one request (101).

Memory is bounded per site: 1,000 latencies, 60 minutes, 100 samples and 300 distinct paths (more paths still count in the totals).

```bash
locdev metrics
locdev metrics shop --json
locdev metrics shop --reset
```

5xx answers and an unreachable server also show up in the [Debug Center](/docs/debug-center).
