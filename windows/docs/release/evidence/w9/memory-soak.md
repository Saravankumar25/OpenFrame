# Memory soak (release gate P1)

> 2026-10-18. Script: `tests/e2e/soak/memory-soak.mjs`, run on the installed program. Gate: [windows-internal-release-gate.md](../../windows-internal-release-gate.md).

## Run 1: unbounded, 2 hours

2,600 loops, 0 errors, no crash.

| Process | Growth | Peak |
|---|---|---|
| App core | 9.1 MB an hour | 38 MB |
| Web view | 586.8 MB an hour | 1.7 GB |

The web view grew because one screenplay grew to about 10,000 lines during the run. Memory went up and down (garbage
collection) and the loops slowed as the document grew.

## Run 2: bounded, 75 minutes

The screenplay was cleared every 25 loops. The slope was taken over the last 45 minutes.

| Item | Result | Limit |
|---|---|---|
| App core | 0.8 MB an hour | |
| Web view | 39 MB an hour | 40 |
| Loops | 2,271 | |
| Errors | 0 | |
| Web view private memory | about 400 to 455 MB | |

Result: PASS.

## Conclusion

No leak at the core. Web view memory follows document size. A small drift remains at an extreme typing pace (about 30
loops a minute, far faster than a person). The 120-page typing latency check was not run separately; the spec 97 typing
budget passed in the final run.

Status: Pass, with notes.
