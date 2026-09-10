# Tasks

## 1. Make the failure visible first

<!--
Tests that pin changed or existing behaviour go here, BEFORE the code. Say what
failure is expected — "verify it fails to compile", "verify it fails with X".
-->

- [ ] 1.1 <!-- write test, verify it FAILS, and say how -->

## 2. <!-- implementation -->

- [ ] 2.1 <!-- task, including how to verify completion -->

## 3. Mutation checks

<!--
For each test asserting a security property: break the property, confirm the
test fails, restore. A test that cannot fail is worse than no test.
-->

- [ ] 3.1 <!-- break X, verify test Y fails, restore -->

## 4. Gates

- [ ] 4.1 Run the test suite; verify every test passes and the count matches
      what CI derives.
- [ ] 4.2 Run `cargo fmt --check` and `clippy -D warnings`; verify both clean.
- [ ] 4.3 Run `openspec validate <change> --type change --strict`.
