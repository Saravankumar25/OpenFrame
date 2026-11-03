# Notes for Apple App Review

Paste into App Store Connect, App Review Information, Notes.

OpenFrame Studio is an offline desktop app for screenwriters and production teams. It has no account, no server of its
own, no analytics and no web payment. All purchases go through Apple.

**How the trial works.** A free in-app purchase named "7-day Trial" starts the trial. The app counts 168 hours from Apple's
date on that purchase. When the trial ends the app locks until the person buys "Lifetime Access", a one-time non-consumable.
Projects stay on the Mac. The first-launch screen states the trial length, what stops working when it ends and the later
charge (Guideline 3.1.1). The price shown comes from StoreKit at run time.

**How to test (sandbox Apple Account).**

1. Open the app. The first-launch screen appears.
2. Choose Start free trial. Apple's sheet appears. Confirm. The app opens and the top bar shows "Trial ends in 7 days".
3. Choose Buy full version. Apple's sheet appears. Confirm. The chip and the button disappear, then three short screens
   appear once: a note about data, a file safety tutorial and a welcome.

**How to restore.** Restore purchase is on the first-launch screen, the lock screen and the check screen.

**Network use.** The app works offline after the first start. The AI assistant is optional: its model is a data file the
person chooses to download, in Settings, from Hugging Face. The AI engine itself is inside the app and nothing executable is
downloaded after install. The engine helper is sandboxed (inherits) and listens on 127.0.0.1 only.

**Clock check.** The app reads the Mac's clock to count the trial. If the clock is set back behind a date it already trusted,
a screen asks to set the correct time. It clears by itself once the clock is right.

**Support.** https://openframestudio.app/contact/ (Help, Contact OpenFrame Support).
