# rp2350-security-lab

Embedded security firmware for the Raspberry Pi Pico 2 (RP2350), written in Rust, protected by a DevSecOps CI/CD pipeline that gates every commit through static analysis, SBOM generation, and dependency vulnerability scanning.

**[Live pipeline demo →](https://r221.github.io/rp2350-security-lab/)** · built as a hands-on learning project in embedded security, Rust, and DevSecOps.

---

## What this is

An end-to-end example of the two halves of secure embedded development.

1. **The firmware** — a tamper-response routine that holds a secret and zeroizes it the instant a physical tamper switch trips. Written in Rust for the RP2350's Cortex-M33.
2. **The pipeline** — a GitHub Actions workflow that treats security as an enforced gate, not an afterthought: static analysis, software bill of materials, and vulnerability scanning run on every push, and failing checks block the merge.

---

## The firmware: tamper response

The device holds a 32-byte secret and watches a GPIO pin wired to a tamper switch. When the enclosure is opened, the switch trips and the firmware **zeroizes the secret**. This is the anti-tamper loop of *detect → respond → zeroize*.

**Wiring**

| Signal | Pin | Meaning |
|--------|-----|---------|
| Tamper switch | GPIO15 (pull-up input) | LOW = closed (safe), HIGH = open (tamper) |
| Status LED | GPIO25 (onboard) | ON = armed, OFF = wiped |

**The zeroize uses volatile writes.**

Why use volatile? A `secret = [0; 32]` can be removed by the compiler through dead-store elimination if it decides the buffer is never read again, so the wipe would actually not happen. `write_volatile` forces the memory to be overwritten, and latches the tampered state so the response is one-way. (Production code would use the [`zeroize`](https://crates.io/crates/zeroize) crate; the volatile approach here makes the mechanism explicit.)

The response is **latched**: once tampered, re-closing the lid does not restore the secret.

---

## The pipeline: security as an enforced gate

Every push runs the following jobs. Findings at high or critical severity **fail the build**, and branch protection blocks the merge until all checks pass; vulnerable code cannot reach `main`.

| Stage | Tool | Category | What it catches |
|-------|------|----------|-----------------|
| Build | `cargo build` | compile | Does the firmware cross-compile for the RP2350 in a clean environment? |
| Clippy | `cargo clippy -- -D warnings` | SAST | Insecure and incorrect patterns in the source itself |
| SBOM | Syft | inventory | Generates a standards-format (SPDX) software bill of materials |
| Vuln scan | Grype | SCA | Dependencies with known CVEs, checked against the SBOM |
| Multi-scan | Trivy | vuln + secret + config | Overlapping vuln detection plus secret and misconfiguration scanning |

**Why two overlapping vulnerability scanners (Grype and Trivy)?** Corroboration and coverage: overlapping vuln detection gives defense in depth, and Trivy adds secret and misconfiguration scanning that Grype doesn't do. Choosing a severity threshold that blocks versus reports is a deliberate trade space consideration between secure development and shipping real risk.

### Pipeline hardening

The pipeline itself is treated as an attack surface.

- **Actions pinned to commit SHAs**, not version tags — a floating tag can be repointed to malicious code if a maintainer account is compromised (the class of supply-chain attack behind incidents like `tj-actions`). Pinning to an immutable SHA means the workflow runs exactly the reviewed commit.
- **Branch protection** requires all security checks to pass and disables direct pushes to `main`, so every change goes through the gated pull-request flow.
- **Fail-secure behavior observed in practice:** Grype refuses to run against a vulnerability database older than a few days, failing the build rather than scanning against stale threat intelligence and falsely reporting clean.

---

## Why it's built this way

The philosophy is shift left: catch security problems at the earliest moment. For embedded systems that may be captured or reverse-engineered, a vulnerability caught at commit time costs minutes; the same vulnerability in a fielded device can cost a mission.

The firmware and the pipeline are two expressions of the same idea. The firmware assumes the adversary gets physical access and focuses on limiting what that access yields (zeroize the secret). The pipeline assumes vulnerable code will be written and focuses on stopping it before it ships. Both are *assume-compromise, limit-the-blast-radius* thinking, applied at different layers.

---

## Tech

- **Target:** Raspberry Pi Pico 2 / RP2350, Arm Cortex-M33, `thumbv8m.main-none-eabihf`
- **Language:** Rust, `#![no_std]`, [`rp235x-hal`](https://github.com/rp-rs/rp-hal)
- **Flashing:** `picotool` over USB (`cargo run`)
- **CI:** GitHub Actions — Clippy, Syft, Grype, Trivy
- **Demo:** static GitHub Pages simulation of the pipeline ([source](./index.html))

---

## Build and flash

```sh
# Add the RP2350 target once
rustup target add thumbv8m.main-none-eabihf

# Build
cargo build

# Flash over USB (Pico in BOOTSEL mode; requires picotool on PATH)
cargo run
```

---

## Status

A learning project, actively evolving. Next directions: persisting the protected secret in flash (so zeroization survives power-cycles, closer to a real stored key), a 3D-printed tamper enclosure driving the switch, and secure-boot provisioning on the RP2350.