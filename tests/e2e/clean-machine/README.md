# Clean-machine proof (level 4)

This proves the Windows installer on a computer that has never seen the project. It runs in the
VirtualBox guest `OpenFrame-W9-Clean`, restored from its `clean` snapshot (Windows 11, no developer
tools, no Visual C++ runtime installed for the whole machine). The host drives the installed app
through WebDriver, so nothing for testing is installed in the guest except two small driver programs.

## What it does

`prepare` restores the snapshot, starts the guest, copies the drivers, both installers and Process
Monitor in, opens a port from the host to the guest's driver, and scans both installers with
Microsoft Defender. Then the steps run in this order:

1. `install-previous`: silent install of the earlier internal build (d271894), with no admin rights.
2. `gate-and-journey`: wrong passwords first (the refusal and the wait), the right one, a project,
   some typing, close, relaunch, check the text is there.
3. `upgrade`: install the final build (995bfac) over it; the project must survive.
4. `gate-bypass`: edit settings files and registry keys to try to skip the password screen. It must
   still show. Everything is restored afterwards.
5. `ai-download`: download OpenFrame AI, then cut the network and ask a question.
6. `offline-export-backup`: with no network, export a PDF, make a backup, restore it as a copy.
7. `power-loss`: type a line, pull the virtual power cord, start again, check the line.
8. `uninstall-keeps-projects`: run the real uninstaller; projects must stay.
9. `reinstall-offline-start`: reinstall with no network; the project must open.
10. `finish`: stop Process Monitor, save its log outside the repo, write the summary.

## How to run

Run it only when you can leave the computer alone: it types into the guest and takes screenshots,
and the download step can take up to 90 minutes.

    node tests/e2e/clean-machine/vm-proof.mjs --dry-run            # show the plan, run nothing
    node tests/e2e/clean-machine/vm-proof.mjs --all                # everything
    node tests/e2e/clean-machine/vm-proof.mjs --step upgrade       # one step (run prepare once first)
    node tests/e2e/clean-machine/vm-proof.mjs --all --keep-going   # carry on after a failed step

Options: `--offline link|firewall|auto`. The default `auto` takes the virtual network cable out; if the
test connection then stops answering, it blocks outgoing traffic with a guest firewall rule instead.

Secrets are read when needed and never printed: the guest account password from
`C:\w9-vm\guest-account.txt`, the team password from `OPENFRAME_TEAM_PASSWORD` (or the Windows User
variable of the same name).

## Where the evidence goes

- `docs/release/evidence/w9/vm/NN-name.png`: numbered screenshots.
- `docs/release/evidence/w9/vm/steps.json`: one entry per step with times, result and notes.
- `docs/release/evidence/w9/vm/defender-*.txt`: Defender scan output for each installer.
- `docs/release/evidence/w9/vm/procmon-summary.md`: network, file and registry activity of the app,
  with anything outside its allowed folders flagged. Made by `procmon-report.mjs`
  (`node procmon-report.mjs --self-test` checks it on a small built-in sample).
- `C:\w9-vm\procmon\session.csv`: the full Process Monitor log. It is large and stays out of the repo.
