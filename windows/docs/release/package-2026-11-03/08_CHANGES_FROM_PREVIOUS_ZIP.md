# 08. Changes from the previous zip (`Trial_Licensing_Mocks.zip`) and why

The previous package was mocks plus a design note. It was checked against the real code on 2026-11-02 and 2026-11-03. This package replaces it.

| Topic | Previous zip | Now | Why |
|---|---|---|---|
| Mac trial | Computed by the app using `AppTransaction.originalPurchaseDate` (first download). Real code instead used a zero-price trial item plus a licence server. | Zero-price trial item, computed offline with a clock guard. No server. | Owner decision: no server and no device binding. The first-download date starts the clock at install, not at the user's choice, and Apple's guideline names the free "XX-day Trial" item. |
| Licence server | Not in the zip; real code had a full axum service and Mac client | Removed everywhere | Owner decision |
| Chip wording | `Trial: N days left` | `Trial ends in N days` | Owner wording |
| Chip placement | Project top bar only | Also on Application Home | Gap found in real code (`AppHome.tsx` had none) |
| Reminders | Over a project | Over a project or on Home | Owner request; the Mac model had a project-only gate |
| Windows refresh | Read once at launch | Refreshes on `OfflineLicensesChanged`, focus, every 15 minutes, midnight and at expiry | Real code never re-read the Store while open |
| Windows refund | Showed "trial ended" | Shows the refund wording (`Revoked`) | Real mapping bug |
| Prices | Never in the app | Windows: none. Mac: the StoreKit-supplied price on the first-launch and lock screens | Apple Guideline 3.1.1 requires the later charge to be disclosed before a trial starts |
| Lock screen links | Only Buy and Restore | Adds `Need help? Contact us` | A locked person needs a way to reach support |
| Contact | Not covered | One link to the website contact page; no form | Owner decision; the privacy page stays true |
| Socials | Placeholder handles shown | Hidden in release builds until real URLs exist | Do not ship fake handles to a store |
| Clock tricks (Mac) | Server time and monotonic clock | Keychain clock guard (G1 to G4), new `ClockChanged` screen (mock 201) | Needed once the server is gone |
| Update safety | Not covered | File 05 audit and actions | Owner question; the code already has strong migration safety, with gaps listed |
| Placeholders | Informal | Named `REPLACE_WITH_...`, release guard, file 06 | Identity values arrive after verification |
| AI re-harness | Separate file | Included as Track B | Owner request |
| Mocks | 182 to 198 | 182 to 203 (Home chip, clock screen, contact) | Matches the new design |

Unchanged and still valid from the previous zip: the data note (6 points), the 4-step tutorial, the welcome popup order, the logo and theme notes, the "once only and in order" rule after purchase, "Windows has no first-launch choice screen", internal builds show nothing.

Mock files carried forward with small edits are listed in `Trial_Licensing_Mocks/Straft_Trial_Licensing_And_FirstRun_Changes.md`, section 1.
