Release Checklist
-----------------
This checklist is based on ripgrep release process . 

* Ensure local `master` is up to date with respect to `origin/master`.
* Make sure that `rustc --version` matches MSRV. 
* Run `cargo update` and review dependency updates. Commit updated
  `Cargo.lock`.
* Run `cargo outdated -d 1` and review semver incompatible updates. Unless there is
  a strong motivation otherwise, review and update every dependency.
* Run `cargo test` or `cargo msrv` to check if MSRV needs to be
  bumped. If MSRV must be updated, update `rust-version` key in Cargo.toml as well
  as the MSRV version mentioned in the readme. 
* Update the CHANGELOG as appropriate.
* Edit the `Cargo.toml` to set the new jolly version. Run
  `cargo update -p jolly` so that the `Cargo.lock` is updated. Commit the
  changes and create a new signed tag. 
* Push changes to GitHub, NOT including the tag. (But do not publish new
  version of jolly to crates.io yet.)
* Once CI for `master` finishes successfully, push the version tag. (Trying to
  do this in one step seems to result in GitHub Actions not seeing the tag
  push and thus not running the release workflow.)
* Wait for CI to finish creating the release. If the release build fails, then
  delete the tag from GitHub, make fixes, re-tag, delete the release and push.
* Copy the relevant section of the CHANGELOG to the tagged release notes.
  Include this blurb describing what jolly is:
  > tbd
* Run `cargo publish`.
* Add TBD section to the top of the CHANGELOG:
  ```
  TBD
  ===
  Unreleased changes. Release notes have not yet been written.
  ```
