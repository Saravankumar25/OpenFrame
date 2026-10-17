# Bug report template

Copy this text into an email or a message. One problem per report. Plain words are fine.

```text
Title: (one line, what went wrong)

Who and where
  Your name:
  Computer: Windows 11 / Mac (which chip, which macOS)
  OpenFrame version and build date (Settings, About):
  Installer file name and SHA-256 (from the file name and the value the owner sent):
  Memory of the computer (if you know):

What you were doing (steps, in order)
  1.
  2.
  3.

What you expected to happen:

What happened instead:

How bad is it? (pick the first that fits)
  [ ] Loses data
  [ ] Corrupts a project
  [ ] Fails a migration or a recovery
  [ ] The AI changed my project on its own
  [ ] Exposes private notes
  [ ] Other: I could not finish what I wanted / it was confusing, slow or ugly / a wish

Does it happen every time? (every time / sometimes / once)

Did you lose any text or data? (yes / no / not sure)

Attachments (please add if you can)
  [ ] A screenshot
  [ ] The diagnostics file (Settings, Project settings, Support, Export Diagnostics)
  [ ] The AI report (Settings, Offline AI, AI health, Export AI report), if the AI is involved
  [ ] Logs attached: yes / no
  [ ] A sample script that shows the problem (do not send your real script)
```

## For the person collecting reports

| Field | Fill in |
|---|---|
| Report number | Next number in the tracking sheet |
| Severity | After reading, using the classes in the [triage page](pilot-kit/triage.md) |
| Type | Defect, installation restriction, idea, question |
| Reproduced by | Name and date, or "not reproduced" |
| Cause | The file and function once known |
| Fix | Commit and test name |
| Checked again by | Reporter and date, in the build number |
