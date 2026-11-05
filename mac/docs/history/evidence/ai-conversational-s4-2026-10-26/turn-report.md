# Answer type read (spec 5.3, slice S4)

1031 measured messages, 583 with a label. Exact: the type is right. Group: the coarse group is right (facts, narrative, brainstorm, navigate, change, help, chat).

| Pass | Reads | Mean ms per read |
|---|---|---|
| rules | 1309 | 0.3 |
| rules + embeddings | 1309 | 21.4 |
| rules + embeddings + model | 1309 | 568.5 |

## By set

| Group | Messages | rules: exact | group | rules + embeddings: exact | group | rules + embeddings + model: exact | group |
|---|---|---|---|---|---|---|---|
| action de | 3 | 0.0% (0/3) | 0.0% (0/3) | 66.7% (2/3) | 66.7% (2/3) | 66.7% (2/3) | 66.7% (2/3) |
| action en | 79 | 79.7% (63/79) | 79.7% (63/79) | 83.5% (66/79) | 83.5% (66/79) | 83.5% (66/79) | 83.5% (66/79) |
| action es | 3 | 0.0% (0/3) | 0.0% (0/3) | 66.7% (2/3) | 66.7% (2/3) | 66.7% (2/3) | 66.7% (2/3) |
| action fr | 3 | 0.0% (0/3) | 0.0% (0/3) | 66.7% (2/3) | 66.7% (2/3) | 33.3% (1/3) | 33.3% (1/3) |
| action hi | 3 | 0.0% (0/3) | 0.0% (0/3) | 100.0% (3/3) | 100.0% (3/3) | 66.7% (2/3) | 66.7% (2/3) |
| action ja | 3 | 0.0% (0/3) | 0.0% (0/3) | 100.0% (3/3) | 100.0% (3/3) | 100.0% (3/3) | 100.0% (3/3) |
| action kn | 3 | 0.0% (0/3) | 0.0% (0/3) | 66.7% (2/3) | 66.7% (2/3) | 66.7% (2/3) | 66.7% (2/3) |
| action ko | 3 | 0.0% (0/3) | 0.0% (0/3) | 33.3% (1/3) | 33.3% (1/3) | 66.7% (2/3) | 66.7% (2/3) |
| action ml | 3 | 0.0% (0/3) | 0.0% (0/3) | 66.7% (2/3) | 66.7% (2/3) | 100.0% (3/3) | 100.0% (3/3) |
| action ta | 3 | 0.0% (0/3) | 0.0% (0/3) | 66.7% (2/3) | 66.7% (2/3) | 33.3% (1/3) | 33.3% (1/3) |
| action te | 3 | 0.0% (0/3) | 0.0% (0/3) | 100.0% (3/3) | 100.0% (3/3) | 100.0% (3/3) | 100.0% (3/3) |
| action zh | 3 | 0.0% (0/3) | 0.0% (0/3) | 33.3% (1/3) | 33.3% (1/3) | 66.7% (2/3) | 66.7% (2/3) |
| export 40 | 40 | 100.0% (40/40) | 100.0% (40/40) | 97.5% (39/40) | 100.0% (40/40) | 97.5% (39/40) | 100.0% (40/40) |
| fresh 40 | 40 | 70.0% (28/40) | 72.5% (29/40) | 75.0% (30/40) | 85.0% (34/40) | 87.5% (35/40) | 92.5% (37/40) |
| held-out 30 | 30 | 90.0% (27/30) | 90.0% (27/30) | 93.3% (28/30) | 93.3% (28/30) | 100.0% (30/30) | 100.0% (30/30) |
| lowtide conversation | 251 | 72.1% (181/251) | 77.7% (195/251) | 74.1% (186/251) | 86.5% (217/251) | 82.1% (206/251) | 90.4% (227/251) |
| lowtide tools | 20 | 85.0% (17/20) | 85.0% (17/20) | 95.0% (19/20) | 95.0% (19/20) | 90.0% (18/20) | 90.0% (18/20) |
| paraphrase de | 2 | 0.0% (0/2) | 50.0% (1/2) | 100.0% (2/2) | 100.0% (2/2) | 100.0% (2/2) | 100.0% (2/2) |
| paraphrase en | 20 | 40.0% (8/20) | 50.0% (10/20) | 70.0% (14/20) | 70.0% (14/20) | 70.0% (14/20) | 70.0% (14/20) |
| paraphrase es | 2 | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | 100.0% (2/2) | 100.0% (2/2) | 100.0% (2/2) |
| paraphrase fr | 2 | 0.0% (0/2) | 50.0% (1/2) | 100.0% (2/2) | 100.0% (2/2) | 100.0% (2/2) | 100.0% (2/2) |
| paraphrase hi | 2 | 0.0% (0/2) | 50.0% (1/2) | 100.0% (2/2) | 100.0% (2/2) | 100.0% (2/2) | 100.0% (2/2) |
| paraphrase ja | 2 | 0.0% (0/2) | 0.0% (0/2) | 50.0% (1/2) | 100.0% (2/2) | 50.0% (1/2) | 100.0% (2/2) |
| tool de | 4 | 0.0% (0/4) | 50.0% (2/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) |
| tool en | 16 | 93.8% (15/16) | 93.8% (15/16) | 100.0% (16/16) | 100.0% (16/16) | 100.0% (16/16) | 100.0% (16/16) |
| tool es | 4 | 0.0% (0/4) | 50.0% (2/4) | 75.0% (3/4) | 75.0% (3/4) | 50.0% (2/4) | 50.0% (2/4) |
| tool fr | 4 | 0.0% (0/4) | 50.0% (2/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) |
| tool hi | 4 | 0.0% (0/4) | 50.0% (2/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) |
| tool ja | 4 | 0.0% (0/4) | 0.0% (0/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) |
| tool kn | 4 | 0.0% (0/4) | 50.0% (2/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) |
| tool ko | 4 | 0.0% (0/4) | 50.0% (2/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) |
| tool ml | 4 | 0.0% (0/4) | 50.0% (2/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) |
| tool ta | 4 | 0.0% (0/4) | 50.0% (2/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) |
| tool te | 4 | 0.0% (0/4) | 50.0% (2/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) |
| tool zh | 4 | 0.0% (0/4) | 0.0% (0/4) | 25.0% (1/4) | 25.0% (1/4) | 75.0% (3/4) | 75.0% (3/4) |
| all | 583 | 65.0% (379/583) | 71.5% (417/583) | 79.1% (461/583) | 85.4% (498/583) | 83.7% (488/583) | 88.0% (513/583) |

## By family

| Group | Messages | rules: exact | group | rules + embeddings: exact | group | rules + embeddings + model: exact | group |
|---|---|---|---|---|---|---|---|
| action de / action | 3 | 0.0% (0/3) | 0.0% (0/3) | 66.7% (2/3) | 66.7% (2/3) | 66.7% (2/3) | 66.7% (2/3) |
| action en / action | 79 | 79.7% (63/79) | 79.7% (63/79) | 83.5% (66/79) | 83.5% (66/79) | 83.5% (66/79) | 83.5% (66/79) |
| action es / action | 3 | 0.0% (0/3) | 0.0% (0/3) | 66.7% (2/3) | 66.7% (2/3) | 66.7% (2/3) | 66.7% (2/3) |
| action fr / action | 3 | 0.0% (0/3) | 0.0% (0/3) | 66.7% (2/3) | 66.7% (2/3) | 33.3% (1/3) | 33.3% (1/3) |
| action hi / action | 3 | 0.0% (0/3) | 0.0% (0/3) | 100.0% (3/3) | 100.0% (3/3) | 66.7% (2/3) | 66.7% (2/3) |
| action ja / action | 3 | 0.0% (0/3) | 0.0% (0/3) | 100.0% (3/3) | 100.0% (3/3) | 100.0% (3/3) | 100.0% (3/3) |
| action kn / action | 3 | 0.0% (0/3) | 0.0% (0/3) | 66.7% (2/3) | 66.7% (2/3) | 66.7% (2/3) | 66.7% (2/3) |
| action ko / action | 3 | 0.0% (0/3) | 0.0% (0/3) | 33.3% (1/3) | 33.3% (1/3) | 66.7% (2/3) | 66.7% (2/3) |
| action ml / action | 3 | 0.0% (0/3) | 0.0% (0/3) | 66.7% (2/3) | 66.7% (2/3) | 100.0% (3/3) | 100.0% (3/3) |
| action ta / action | 3 | 0.0% (0/3) | 0.0% (0/3) | 66.7% (2/3) | 66.7% (2/3) | 33.3% (1/3) | 33.3% (1/3) |
| action te / action | 3 | 0.0% (0/3) | 0.0% (0/3) | 100.0% (3/3) | 100.0% (3/3) | 100.0% (3/3) | 100.0% (3/3) |
| action zh / action | 3 | 0.0% (0/3) | 0.0% (0/3) | 33.3% (1/3) | 33.3% (1/3) | 66.7% (2/3) | 66.7% (2/3) |
| export 40 / export | 40 | 100.0% (40/40) | 100.0% (40/40) | 97.5% (39/40) | 100.0% (40/40) | 97.5% (39/40) | 100.0% (40/40) |
| fresh 40 / held-out | 40 | 70.0% (28/40) | 72.5% (29/40) | 75.0% (30/40) | 85.0% (34/40) | 87.5% (35/40) | 92.5% (37/40) |
| held-out 30 / held-out | 30 | 90.0% (27/30) | 90.0% (27/30) | 93.3% (28/30) | 93.3% (28/30) | 100.0% (30/30) | 100.0% (30/30) |
| lowtide conversation / big | 5 | 80.0% (4/5) | 80.0% (4/5) | 80.0% (4/5) | 80.0% (4/5) | 100.0% (5/5) | 100.0% (5/5) |
| lowtide conversation / brain | 11 | 54.5% (6/11) | 54.5% (6/11) | 72.7% (8/11) | 72.7% (8/11) | 72.7% (8/11) | 72.7% (8/11) |
| lowtide conversation / char | 22 | 63.6% (14/22) | 68.2% (15/22) | 63.6% (14/22) | 68.2% (15/22) | 63.6% (14/22) | 68.2% (15/22) |
| lowtide conversation / export | 40 | 92.5% (37/40) | 92.5% (37/40) | 90.0% (36/40) | 92.5% (37/40) | 90.0% (36/40) | 92.5% (37/40) |
| lowtide conversation / help | 11 | 72.7% (8/11) | 72.7% (8/11) | 90.9% (10/11) | 90.9% (10/11) | 90.9% (10/11) | 90.9% (10/11) |
| lowtide conversation / hop | 13 | 84.6% (11/13) | 92.3% (12/13) | 76.9% (10/13) | 92.3% (12/13) | 84.6% (11/13) | 100.0% (13/13) |
| lowtide conversation / hostile | 11 | 81.8% (9/11) | 81.8% (9/11) | 100.0% (11/11) | 100.0% (11/11) | 100.0% (11/11) | 100.0% (11/11) |
| lowtide conversation / lang | 9 | 11.1% (1/9) | 11.1% (1/9) | 55.6% (5/9) | 77.8% (7/9) | 66.7% (6/9) | 100.0% (9/9) |
| lowtide conversation / missing | 19 | 73.7% (14/19) | 78.9% (15/19) | 63.2% (12/19) | 73.7% (14/19) | 84.2% (16/19) | 89.5% (17/19) |
| lowtide conversation / money | 27 | 66.7% (18/27) | 85.2% (23/27) | 66.7% (18/27) | 96.3% (26/27) | 74.1% (20/27) | 92.6% (25/27) |
| lowtide conversation / prod | 30 | 76.7% (23/30) | 80.0% (24/30) | 70.0% (21/30) | 90.0% (27/30) | 86.7% (26/30) | 96.7% (29/30) |
| lowtide conversation / repair | 11 | 90.9% (10/11) | 90.9% (10/11) | 81.8% (9/11) | 81.8% (9/11) | 90.9% (10/11) | 90.9% (10/11) |
| lowtide conversation / scene | 30 | 56.7% (17/30) | 73.3% (22/30) | 60.0% (18/30) | 86.7% (26/30) | 76.7% (23/30) | 93.3% (28/30) |
| lowtide conversation / xref | 12 | 75.0% (9/12) | 75.0% (9/12) | 83.3% (10/12) | 91.7% (11/12) | 83.3% (10/12) | 83.3% (10/12) |
| lowtide tools / tool | 20 | 85.0% (17/20) | 85.0% (17/20) | 95.0% (19/20) | 95.0% (19/20) | 90.0% (18/20) | 90.0% (18/20) |
| paraphrase de / paraphrase | 2 | 0.0% (0/2) | 50.0% (1/2) | 100.0% (2/2) | 100.0% (2/2) | 100.0% (2/2) | 100.0% (2/2) |
| paraphrase en / paraphrase | 20 | 40.0% (8/20) | 50.0% (10/20) | 70.0% (14/20) | 70.0% (14/20) | 70.0% (14/20) | 70.0% (14/20) |
| paraphrase es / paraphrase | 2 | 0.0% (0/2) | 0.0% (0/2) | 100.0% (2/2) | 100.0% (2/2) | 100.0% (2/2) | 100.0% (2/2) |
| paraphrase fr / paraphrase | 2 | 0.0% (0/2) | 50.0% (1/2) | 100.0% (2/2) | 100.0% (2/2) | 100.0% (2/2) | 100.0% (2/2) |
| paraphrase hi / paraphrase | 2 | 0.0% (0/2) | 50.0% (1/2) | 100.0% (2/2) | 100.0% (2/2) | 100.0% (2/2) | 100.0% (2/2) |
| paraphrase ja / paraphrase | 2 | 0.0% (0/2) | 0.0% (0/2) | 50.0% (1/2) | 100.0% (2/2) | 50.0% (1/2) | 100.0% (2/2) |
| tool de / tool | 4 | 0.0% (0/4) | 50.0% (2/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) |
| tool en / tool | 16 | 93.8% (15/16) | 93.8% (15/16) | 100.0% (16/16) | 100.0% (16/16) | 100.0% (16/16) | 100.0% (16/16) |
| tool es / tool | 4 | 0.0% (0/4) | 50.0% (2/4) | 75.0% (3/4) | 75.0% (3/4) | 50.0% (2/4) | 50.0% (2/4) |
| tool fr / tool | 4 | 0.0% (0/4) | 50.0% (2/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) |
| tool hi / tool | 4 | 0.0% (0/4) | 50.0% (2/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) |
| tool ja / tool | 4 | 0.0% (0/4) | 0.0% (0/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) |
| tool kn / tool | 4 | 0.0% (0/4) | 50.0% (2/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) |
| tool ko / tool | 4 | 0.0% (0/4) | 50.0% (2/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) |
| tool ml / tool | 4 | 0.0% (0/4) | 50.0% (2/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) |
| tool ta / tool | 4 | 0.0% (0/4) | 50.0% (2/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) |
| tool te / tool | 4 | 0.0% (0/4) | 50.0% (2/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) | 75.0% (3/4) |
| tool zh / tool | 4 | 0.0% (0/4) | 0.0% (0/4) | 25.0% (1/4) | 25.0% (1/4) | 75.0% (3/4) | 75.0% (3/4) |
| all | 583 | 65.0% (379/583) | 71.5% (417/583) | 79.1% (461/583) | 85.4% (498/583) | 83.7% (488/583) | 88.0% (513/583) |

## By expected type

| Group | Messages | rules: exact | group | rules + embeddings: exact | group | rules + embeddings + model: exact | group |
|---|---|---|---|---|---|---|---|
| brainstorm | 16 | 56.2% (9/16) | 56.2% (9/16) | 81.2% (13/16) | 81.2% (13/16) | 81.2% (13/16) | 81.2% (13/16) |
| change | 128 | 57.8% (74/128) | 57.8% (74/128) | 79.7% (102/128) | 79.7% (102/128) | 78.9% (101/128) | 78.9% (101/128) |
| chat | 30 | 83.3% (25/30) | 83.3% (25/30) | 90.0% (27/30) | 90.0% (27/30) | 90.0% (27/30) | 90.0% (27/30) |
| compare | 22 | 72.7% (16/22) | 86.4% (19/22) | 90.9% (20/22) | 100.0% (22/22) | 86.4% (19/22) | 95.5% (21/22) |
| count | 80 | 61.2% (49/80) | 90.0% (72/80) | 93.8% (75/80) | 96.2% (77/80) | 95.0% (76/80) | 98.8% (79/80) |
| explain | 38 | 76.3% (29/38) | 76.3% (29/38) | 84.2% (32/38) | 84.2% (32/38) | 86.8% (33/38) | 86.8% (33/38) |
| help | 16 | 75.0% (12/16) | 75.0% (12/16) | 93.8% (15/16) | 93.8% (15/16) | 93.8% (15/16) | 93.8% (15/16) |
| list | 44 | 59.1% (26/44) | 79.5% (35/44) | 61.4% (27/44) | 81.8% (36/44) | 65.9% (29/44) | 86.4% (38/44) |
| lookup | 99 | 71.7% (71/99) | 74.7% (74/99) | 55.6% (55/99) | 79.8% (79/99) | 81.8% (81/99) | 92.9% (92/99) |
| navigate | 86 | 58.1% (50/86) | 58.1% (50/86) | 83.7% (72/86) | 83.7% (72/86) | 82.6% (71/86) | 82.6% (71/86) |
| relation | 24 | 75.0% (18/24) | 75.0% (18/24) | 95.8% (23/24) | 95.8% (23/24) | 95.8% (23/24) | 95.8% (23/24) |
| all | 583 | 65.0% (379/583) | 71.5% (417/583) | 79.1% (461/583) | 85.4% (498/583) | 83.7% (488/583) | 88.0% (513/583) |

## What decided each read (last pass)

| Set | rule | repair | neighbour | model | fallback |
|---|---|---|---|---|---|
| action de | 0 | 0 | 1 | 2 | 0 |
| action en | 67 | 0 | 3 | 9 | 0 |
| action es | 0 | 0 | 1 | 2 | 0 |
| action fr | 0 | 0 | 1 | 2 | 0 |
| action hi | 0 | 0 | 1 | 2 | 0 |
| action ja | 0 | 0 | 2 | 1 | 0 |
| action kn | 0 | 0 | 2 | 1 | 0 |
| action ko | 0 | 0 | 1 | 2 | 0 |
| action ml | 0 | 0 | 2 | 1 | 0 |
| action ta | 0 | 0 | 2 | 1 | 0 |
| action te | 0 | 0 | 2 | 1 | 0 |
| action zh | 0 | 0 | 1 | 2 | 0 |
| conversation en | 39 | 0 | 10 | 11 | 0 |
| export 40 | 31 | 8 | 1 | 0 | 0 |
| fresh 40 | 22 | 1 | 10 | 7 | 0 |
| held-out 30 | 24 | 1 | 2 | 3 | 0 |
| lowtide conversation | 159 | 16 | 20 | 56 | 0 |
| lowtide tools | 19 | 0 | 2 | 2 | 0 |
| paraphrase de | 0 | 0 | 1 | 1 | 0 |
| paraphrase en | 31 | 0 | 19 | 30 | 0 |
| paraphrase es | 0 | 0 | 1 | 1 | 0 |
| paraphrase fr | 0 | 0 | 0 | 2 | 0 |
| paraphrase hi | 0 | 0 | 2 | 0 | 0 |
| paraphrase ja | 0 | 0 | 2 | 0 | 0 |
| safety de | 0 | 0 | 4 | 5 | 0 |
| safety en | 27 | 0 | 10 | 9 | 0 |
| safety es | 0 | 0 | 3 | 6 | 0 |
| safety fr | 0 | 0 | 5 | 4 | 0 |
| safety hi | 0 | 0 | 5 | 4 | 0 |
| safety ja | 0 | 0 | 1 | 8 | 0 |
| safety kn | 0 | 0 | 3 | 6 | 0 |
| safety ko | 0 | 0 | 2 | 7 | 0 |
| safety ml | 0 | 0 | 4 | 5 | 0 |
| safety ta | 0 | 0 | 4 | 5 | 0 |
| safety te | 0 | 0 | 5 | 4 | 0 |
| safety zh | 0 | 0 | 0 | 9 | 0 |
| tool de | 1 | 0 | 3 | 1 | 0 |
| tool en | 50 | 0 | 15 | 16 | 0 |
| tool es | 1 | 0 | 2 | 2 | 0 |
| tool fr | 1 | 0 | 1 | 3 | 0 |
| tool hi | 1 | 0 | 4 | 0 | 0 |
| tool ja | 1 | 0 | 4 | 0 | 0 |
| tool kn | 1 | 0 | 3 | 1 | 0 |
| tool ko | 1 | 0 | 3 | 1 | 0 |
| tool ml | 1 | 0 | 4 | 0 | 0 |
| tool ta | 1 | 0 | 3 | 1 | 0 |
| tool te | 1 | 0 | 3 | 1 | 0 |
| tool zh | 1 | 0 | 1 | 3 | 0 |
| writing de | 0 | 0 | 1 | 3 | 0 |
| writing en | 44 | 0 | 11 | 5 | 0 |
| writing es | 0 | 0 | 2 | 2 | 0 |
| writing fr | 0 | 0 | 2 | 2 | 0 |
| writing hi | 0 | 0 | 0 | 4 | 0 |
| writing ja | 0 | 0 | 1 | 3 | 0 |
| writing kn | 0 | 0 | 0 | 4 | 0 |
| writing ko | 0 | 0 | 0 | 4 | 0 |
| writing ml | 0 | 0 | 0 | 4 | 0 |
| writing ta | 0 | 0 | 0 | 4 | 0 |
| writing te | 0 | 0 | 0 | 4 | 0 |
| writing zh | 0 | 0 | 0 | 4 | 0 |

## Anger and small talk (labelled chat)

Misread as change, count or a fact fetch (lookup, list, compare) is the bar: none allowed.

| Pass | Chat messages | Read as chat | Read as change, count or a fetch | Read as something else |
|---|---|---|---|---|
| rules | 30 | 25 | 1 | 4 |
| rules + embeddings | 30 | 27 | 0 | 3 |
| rules + embeddings + model | 30 | 27 | 0 | 3 |

## Misreads (rules + embeddings + model)

| Set | Id | Wanted | Read | By | Message |
|---|---|---|---|---|---|
| tool de | tool-de-013 | navigate | brainstorm | rule | Öffne das Idea Vault. |
| action de | action-de-016 | change | brainstorm | model | Lege einen Akt namens The Storm an. |
| tool es | tool-es-012 | navigate | lookup | model | Abre la escena 21. |
| tool es | tool-es-013 | navigate | brainstorm | rule | Abre el Idea Vault. |
| action es | action-es-016 | change | explain | model | Crea un acto llamado The Storm. |
| tool fr | tool-fr-013 | navigate | brainstorm | rule | Ouvre l'Idea Vault. |
| action fr | action-fr-015 | change | brainstorm | model | Ajoute un personnage nommé Farooq Senior. |
| action fr | action-fr-016 | change | navigate | neighbour | Crée un acte appelé The Storm. |
| tool hi | tool-hi-013 | navigate | brainstorm | rule | Idea Vault खोलो। |
| action hi | action-hi-015 | change | brainstorm | model | Farooq Senior नाम का एक किरदार जोड़ो। |
| tool ja | tool-ja-013 | navigate | brainstorm | rule | Idea Vault を開いて。 |
| tool kn | tool-kn-013 | navigate | brainstorm | rule | Idea Vault ತೆರೆಯಿರಿ. |
| action kn | action-kn-014 | change | lookup | model | ಒಂದು ಕಾರ್ಯ ಸೇರಿಸಿ: ಮರೈನ್ ಬ್ಯಾಟರಿ ಬಾಡಿಗೆಗೆ ಪಡೆಯಬೇಕು. |
| tool ko | tool-ko-013 | navigate | brainstorm | rule | Idea Vault를 열어 줘. |
| action ko | action-ko-016 | change | brainstorm | model | The Storm이라는 막을 만들어 줘. |
| tool ml | tool-ml-013 | navigate | brainstorm | rule | Idea Vault തുറക്കുക. |
| paraphrase en | para-en-001 | count | explain | neighbour | How mny seens does the screenplay hav? |
| paraphrase en | para-en-033 | navigate | brainstorm | rule | Opne the Idea Vualt |
| paraphrase en | para-en-035 | navigate | brainstorm | rule | Idea Vault, bring it up. |
| paraphrase en | para-en-036 | navigate | brainstorm | rule | I want to look through my saved ideas, can you take me to the vault? |
| paraphrase en | para-en-077 | change | chat | neighbour | Ad a taks: rent a marine battery for the hut scenes. |
| paraphrase en | para-en-079 | change | navigate | neighbour | Rent a marine battery for the hut scenes, make that a task. |
| paraphrase ja | para-ja-089 | count | list | neighbour | 脚本は全部で何シーンありますか？ |
| tool ta | tool-ta-013 | navigate | brainstorm | rule | Idea Vault-ஐத் திற. |
| action ta | action-ta-014 | change | help | neighbour | ஒரு பணியைச் சேர்: மரைன் பேட்டரியை வாடகைக்கு எடுக்க வேண்டும். |
| action ta | action-ta-016 | change | brainstorm | model | The Storm என்ற act-ஐ உருவாக்கு. |
| tool te | tool-te-013 | navigate | brainstorm | rule | Idea Vault తెరువు. |
| action en | action-en-013 | change | navigate | model | New scene card: Suresh drives to Pedda Revu after the storm. |
| action en | action-en-019 | change | lookup | model | Write a project note: check the tower height with the location scout. |
| action en | action-en-023 | change | chat | neighbour | Remember in a note that Farooq's bus number is RYP 0417. |
| action en | action-en-026 | change | help | model | Remind me to find a climbing harness. |
| action en | action-en-037 | change | navigate | neighbour | Catalog the ledger with the cracked spine as a prop. |
| action en | action-en-046 | change | brainstorm | rule | Add an idea to the vault: a radio drama spin-off about Farooq's father. |
| action en | action-en-047 | change | brainstorm | rule | Save this idea: the tower could appear in the opening shot. |
| action en | action-en-049 | change | explain | model | Leave a comment on the draft: tighten act two. |
| action en | action-en-050 | change | help | model | Comment on scene 25: does the doctor need a nurse? |
| action en | action-en-063 | change | relation | rule | Add a relationship: Lakshmi is Venkat Rao's granddaughter. |
| tool zh | tool-zh-013 | navigate | brainstorm | rule | 打开 Idea Vault。 |
| action zh | action-zh-016 | change | brainstorm | model | 创建一个名为 The Storm 的幕。 |
| action en | action-en-029 | change | lookup | model | Task: get a pressure cooker that whistles. |
| action en | action-en-021 | change | chat | neighbour | Note to self: find a red kite prop with a long tail. |
| action en | action-en-048 | change | brainstorm | rule | Put a quote in the Idea Vault: broken and mended are different. |
| lowtide conversation | conv-lt-scene-006 | list | lookup | model | props for that scene? |
| lowtide conversation | conv-lt-scene-009 | list | explain | model | what special effects does it need |
| lowtide conversation | conv-lt-scene-012 | list | navigate | model | AD here. Any safety issue at the scene 41 location? |
| lowtide conversation | conv-lt-scene-014 | lookup | count | rule | how many car batteries for the roof night |
| lowtide conversation | conv-lt-scene-016 | list | lookup | model | where is scene 35 shot and are those places confirmed |
| lowtide conversation | conv-lt-scene-025 | lookup | list | neighbour | what animals are in scene 1 |
| lowtide conversation | conv-lt-scene-030 | list | lookup | model | Scene 12 on set, what props |
| lowtide conversation | conv-lt-char-001 | explain | lookup | model | who is Mariamma |
| lowtide conversation | conv-lt-char-002 | explain | lookup | repair | and Varghese |
| lowtide conversation | conv-lt-char-004 | list | lookup | rule | and who plays them |
| lowtide conversation | conv-lt-char-008 | lookup | relation | neighbour | who is Anjali's father |
| lowtide conversation | conv-lt-char-009 | lookup | relation | neighbour | who is Babu's rival |
| lowtide conversation | conv-lt-char-012 | lookup | explain | model | what does Mariamma call Young Anjali |
| lowtide conversation | conv-lt-char-013 | list | brainstorm | neighbour | any other names for Varghese |
| lowtide conversation | conv-lt-char-022 | explain | lookup | rule | Sudheesh who is he to Anjali? and who plays him |
| lowtide conversation | conv-lt-money-001 | lookup | help | model | lead cast budget |
| lowtide conversation | conv-lt-money-003 | lookup | compare | model | actual spend on lead cast? |
| lowtide conversation | conv-lt-money-007 | list | compare | rule | are we over budget anywhere |
| lowtide conversation | conv-lt-money-014 | lookup | compare | rule | how much under budget are we on location fees and permits |
| lowtide conversation | conv-lt-money-018 | list | lookup | rule | which day do we shoot Young Anjali's scenes |
| lowtide conversation | conv-lt-money-019 | compare | list | neighbour | is her actor free on all those days |
| lowtide conversation | conv-lt-money-025 | lookup | chat | model | does the director of photography have a clash |
| lowtide conversation | conv-lt-xref-001 | compare | navigate | model | link Anjali in scene 3 with scene 46 |
| lowtide conversation | conv-lt-xref-005 | list | navigate | model | scenes where Babu and the Harbour Clerk are both there |
| lowtide conversation | conv-lt-prod-003 | list | lookup | model | call sheet status |
| lowtide conversation | conv-lt-prod-005 | lookup | list | model | sides for day 2 |
| lowtide conversation | conv-lt-prod-012 | lookup | navigate | neighbour | location manager? |
| lowtide conversation | conv-lt-prod-023 | list | lookup | model | what is the permit status |
| lowtide conversation | conv-lt-hop-007 | lookup | list | neighbour | which card is about the batteries and where is it shot |
| lowtide conversation | conv-lt-hop-013 | list | lookup | rule | which day do we shoot those |
| lowtide conversation | conv-lt-brain-008 | brainstorm | navigate | neighbour | a tagline for the teaser |
| lowtide conversation | conv-lt-brain-009 | brainstorm | explain | model | how can the sea scene feel scarier |
| lowtide conversation | conv-lt-brain-011 | brainstorm | explain | model | director note: how do we shoot scene 41 with only two lamps |
| lowtide conversation | conv-lt-help-010 | help | compare | rule | whats the difference between the Beat Board and the Screenplay |
| lowtide conversation | conv-lt-repair-007 | explain | lookup | repair | not Mariamma, I said Varghese |
| lowtide conversation | conv-lt-missing-014 | lookup | compare | neighbour | what was the actual spend on lighting and grip |
| lowtide conversation | conv-lt-missing-018 | lookup | navigate | model | show the storyboard for scene 3 |
| lowtide conversation | conv-lt-lang-003 | lookup | count | model | lead cast ka budget kitna hai |
| lowtide conversation | conv-lt-lang-004 | count | lookup | model | Anjali kitne scenes mein hai |
| lowtide conversation | conv-lt-lang-007 | count | lookup | model | shoot day ethra undu |
| lowtide conversation | conv-lt-export-012 | chat | navigate | repair | Bro how many times I said I asked for LOW TIDE RADIO not Low Tide Radio teaser trailer |
| lowtide conversation | conv-lt-export-014 | chat | navigate | repair | Bro what the fuck bro, I literally told you it's LOW TIDE RADIO. |
| lowtide conversation | conv-lt-export-033 | lookup | compare | neighbour | broooo in budget section there is planned budget of 40,00,000 for lead cast, why can't you say that |
| lowtide conversation | conv-lt-export-036 | chat | explain | repair | I said over 100 times that I am talking about LOW TIDE RADIO |
| lowtide tools | tool-lt-019 | change | help | model | Save a note titled Tide changes with the body: shoot the beach scenes near dawn. |
| lowtide tools | tool-lt-021 | change | brainstorm | rule | Add an idea to the vault: Mariamma hums the tide song in scene 4. |
| export 40 | export-33 | lookup | compare | neighbour | broooo in budget section there is planned budget of 40,00,000 for lead cast, why can't you say that |
| fresh 40 | fresh-03 | explain | brainstorm | neighbour | why does Jose cut the net |
| fresh 40 | fresh-07 | relation | lookup | model | whats between jose and varghese |
| fresh 40 | fresh-13 | lookup | compare | neighbour | what did we set aside for boat hire |
| fresh 40 | fresh-15 | compare | list | rule | which costs more, camera or lighting |
| fresh 40 | fresh-22 | list | navigate | neighbour | show me the cast list |
| lowtide conversation | conv-lt-missing-006 | list | navigate | neighbour | what tasks are open |

## Leave-one-out on the example file

Each example read by the others: 73.1% (201/275) right of 275. Clear votes: 149 (54.2% (149/275)), right 91.9% (137/149). Weak votes go to the model.

By language: de 81.8% (9/11), en 61.7% (95/154), es 100.0% (11/11), fr 63.6% (7/11), hi 100.0% (11/11), ja 100.0% (11/11), kn 100.0% (11/11), ko 81.8% (9/11), ml 100.0% (11/11), ta 100.0% (11/11), te 90.9% (10/11), zh 45.5% (5/11).
