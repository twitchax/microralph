---
id: PRD-0021
title: "Keep Binary Zips in Releases"
status: active
owner: "twitchax"
created: 2026-01-26
updated: 2026-01-26

principles:
- Follow kord's release pattern exactly for naming and structure
- CI artifacts are already zipped on download—no additional zipping needed
- Update README install instructions to match kord-style curl/unzip/chmod workflow
- Keep WASM OCI publish workflow unchanged

references:
- name: kord README (reference pattern)
  url: https://github.com/twitchax/kord#install
- name: GitHub Actions upload-artifact documentation
  url: https://docs.github.com/en/actions/using-workflows/storing-workflow-data-as-artifacts

acceptance_tests:
- id: uat-001
  name: README install instructions show curl/unzip/chmod workflow
  command: grep -q "unzip" README.md && grep -q "chmod" README.md
  uat_status: verified
- id: uat-002
  name: github-release task attaches artifacts from release-artifacts directory
  command: grep -q "release-artifacts" Makefile.toml
  uat_status: unverified
- id: uat-003
  name: README uses target triple naming convention
  command: grep -q "mr_x86_64-unknown-linux-gnu" README.md
  uat_status: verified

tasks:
- id: T-001
  title: Update README install instructions to match kord pattern
  priority: 1
  status: done
  notes: Replace simple curl/chmod with curl/unzip/chmod pattern. Use target triple names like mr_x86_64-unknown-linux-gnu.zip, mr_aarch64-apple-darwin.zip, mr_x86_64-pc-windows-gnu.zip. Follow kord README format exactly.
- id: T-002
  title: Verify github-release task downloads and attaches zipped artifacts
  priority: 2
  status: todo
  notes: The github-release task already attaches files from release-artifacts/. Verify the workflow in AGENTS.md shows downloading CI artifacts before running github-release. No zipping needed—GitHub Actions artifacts come down as zips already.
- id: T-003
  title: Update Windows install instructions for PowerShell
  priority: 3
  status: todo
  notes: Follow kord's PowerShell pattern with iwr and Expand-Archive for Windows users.

---

# Summary

Align microralph's release workflow and install instructions with the kord pattern. Use target-triple naming for binaries (e.g., `mr_x86_64-unknown-linux-gnu.zip`) and provide kord-style install instructions showing `curl -LO`, `unzip`, `chmod +x`, and placement in PATH.

---

# Problem

The current README uses simple binary names (`mr-linux`, `mr-macos`) with direct `curl | chmod` instructions. This differs from the established kord pattern which:
1. Uses target triples in artifact names for precision
2. Provides zipped artifacts for cleaner downloads
3. Shows explicit `unzip` and path placement steps

This inconsistency makes it harder for users familiar with kord to install microralph, and the simpler naming loses target architecture clarity.

---

# Goals

1. Update README install instructions to exactly match kord's pattern (curl/unzip/chmod/mv)
2. Use target triple naming: `mr_x86_64-unknown-linux-gnu.zip`, `mr_aarch64-apple-darwin.zip`, `mr_x86_64-pc-windows-gnu.zip`
3. Ensure github-release workflow correctly attaches downloaded CI artifacts
4. Provide PowerShell instructions for Windows users following kord's pattern

---

# Non-Goals (MVP)

- Modifying CI build workflow (already produces correct artifact names)
- Adding additional zipping steps (GitHub Actions already zips on download)
- Changing WASM OCI publish workflow
- Supporting additional architectures beyond current targets

---

# History

(Entries appended by `mr run` will go below this line.)

## 2026-01-26 — T-001 Completed
- **Task**: Update README install instructions to match kord pattern
- **Status**: ✅ Done
- **Changes**:
  - Updated `README.md` install section to use target triple naming (`mr_x86_64-unknown-linux-gnu.zip`, `mr_aarch64-apple-darwin.zip`, `mr_x86_64-pc-windows-gnu.zip`)
  - Replaced simple `curl -L ... -o mr` pattern with kord-style `curl -LO` / `unzip` / `chmod a+x` workflow
  - Added Windows PowerShell instructions with `iwr` and `Expand-Archive`
  - Reorganized sections: separate Linux and Mac OS sections, moved Cargo under pre-built binaries, cleaned up duplicate sections
  - UAT passed: 344 tests passed, acceptance tests (grep for "unzip", "chmod", target triple names) all verify

- **Opportunistic UAT Verification**:
  - **uat-001** (README install instructions show curl/unzip/chmod workflow): Verified ✅
  - **uat-003** (README uses target triple naming convention): Verified ✅

---