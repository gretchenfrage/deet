DEET (DEploymEnt Tool) is an opinionated tool for publishing crates.

Expected usage:

	deet --help
		Print this information

	deet check [package path]
		Non-destructive dry run to confirm that package is in a clean
		and publishable state.

	deet check [package path] [version number]
		Non-destructive dry run to confirm that package is in a clean
		and publishable state for publishing that version.

	deet publish [package path] [version number]
		Runs the same checks as the previous command, then actually
		uses `cargo publish` to publish it to crates.io.


