# Clean-machine proof on a virtual machine: blocked

> 2026-10-18. Gate rows C1 to C11 stay Open. The owner must pick one of the three options below.

## What was done

- VirtualBox 7.2.20 installed from the official download; SHA-256 matched.
- Windows 11 Enterprise evaluation 25H2 (build 26200.6584, 7,092,807,680 bytes, SHA-256
  `a61adeab895ef5a4db436e0a7011c92a2ff17bb0357f58b13bbc4062e535e7b9`, from the Microsoft Evaluation Center) installed
  unattended with Guest Additions. A snapshot named "clean" was taken.
- The driver script `tests/e2e/clean-machine/vm-proof.mjs` is written. Steps: prepare, install-previous,
  gate-and-journey, upgrade, gate-bypass, ai-download, offline-export-backup, power-loss, uninstall-keeps-projects,
  reinstall-offline-start, then finish with the Process Monitor report. It has not completed a run.

## Why it is blocked

VirtualBox cannot use the processor's virtualisation because Windows' own hypervisor holds it. Memory Integrity
(virtualization-based security) is on. The log says: `HM: HMR3Init: Attempting fall back to NEM: VT-x is not available`.
VirtualBox then runs through the Windows Hypervisor Platform. In that mode the Windows 11 guest stalled repeatedly at
first sign-in: display 0x0, guest service never ready, and a 60-second clock lag per minute while builds ran.

## Options (owner decision needed)

1. Turn Memory Integrity off for a while (Windows Security, Device security, Core isolation; reboot). Rerun the VM proof,
   about 3 hours. Turn it back on afterwards.
2. Create a second Windows user account on this laptop and run install, gate, journey, uninstall and reinstall there.
   It is not a clean machine, but it is a clean user profile.
3. Accept that the team pilot laptops are the clean machines (W10).

## Decision 2026-10-18

The owner chose option 3: the team pilot laptops are the clean machines. W10 carries the clean-machine proof.
The VM and the Windows image are deleted to free disk. The driver script `tests/e2e/clean-machine/vm-proof.mjs` stays
for a later machine where VT-x is free.
