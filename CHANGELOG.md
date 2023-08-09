# Changelog

## [0.3.0] - 2023-08-09

### Added

- Add support for `description` field. You can use this to provide more detail about a jolly entry, beyond its title. [#19](https://github.com/apgoetz/jolly/pull/19)

- Add support for icons. Jolly will look up appropriate icons for files and display them inline. [#18](https://github.com/apgoetz/jolly/issues/18), [#20](https://github.com/apgoetz/jolly/pull/20), [#35](https://github.com/apgoetz/jolly/pull/35)

- Added support for logging facade. Logging can be configured in the [config file](docs/config.md#log). [#30](https://github.com/apgoetz/jolly/pull/30)

- Added basic CLI args to Jolly. Config file can now  be specified as an argument. [#31](https://github.com/apgoetz/jolly/pull/31)


### Changed

- Text shaping uses `iced` Advanced text shaping. Should have better support for non-ascii characters in entries [#25](https://github.com/apgoetz/jolly/pull/25), [#36](https://github.com/apgoetz/jolly/pull/36)

### Fixed

- Cleaned up window resize commands to avoid flashing of window [#26](https://github.com/apgoetz/jolly/pull/26)


## [0.2.0] - 2023-02-06

### Added

- MSRV statement. Jolly will track latest stable rust [#6](https://github.com/apgoetz/jolly/issues/6)
- Settings Support. Jolly now has ability to customize settings [#10](https://github.com/apgoetz/jolly/issues/10)
- Theme Support. Jolly now has ability to specify the colors of its theme [#11](https://github.com/apgoetz/jolly/issues/11) [#13](https://github.com/apgoetz/jolly/issues/13)
- Packaging for NetBSD. via [#12](https://github.com/apgoetz/jolly/issues/12)

### Fixed

- Jolly can show blank / garbage screen on startup on windows [#9](https://github.com/apgoetz/jolly/issues/7)
- Starting Jolly while typing in another application prevents focus on windows [#14](https://github.com/apgoetz/jolly/issues/14)

## [0.1.1] - 2023-01-04

This release fixes a bug on windows release builds where exeuting system commands would cause a console window to appear. 

## [0.1.0] - 2022-12-22

Initial Release
