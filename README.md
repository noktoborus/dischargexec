# dischargexec

`dischargexec` is a lightweight utility that runs a given command **only if** the system’s battery is currently charging (i.e., the device is connected to AC power).  
It is especially useful in combination with idle managers like [`swayidle`](https://github.com/swaywm/swayidle) to automatically shut down or suspend the machine after a period of inactivity – but only when it is plugged in.

---

## Installation

Install from the source:

```bash
git clone https://github.com/noktoborus/dischargexec
cd dischargexec
cargo install --path .
```

---

## Integration with `swayidle`

The most common use case is to combine `dischargexec` with `swayidle`:

```bash
swayidle timeout 600 'dischargexec exec /usr/sbin/poweroff' \
          resume 'dischargexec abort'
```

- After **600 seconds** of inactivity, `swayidle` triggers `dischargexec exec /usr/sbin/poweroff` – the system powers off **only if** it is still charging.
- If the user moves the mouse or presses a key **before** the timeout expires (or even after, depending on the implementation), the `resume` hook calls `dischargexec abort` to cancel the shutdown.
