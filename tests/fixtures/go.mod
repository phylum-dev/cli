module cli/example

go 1.22.2

require (
	github.com/go-chi/chi/v5 v5.0.12
	github.com/rs/zerolog v1.32.0

	example.com/othermodule v1.2.3
	example.com/othermodule v1.2.4
	example.com/othermodule v1.2.5
	example.com/replacedmodule v1.2.3

	example.com/thismodule v1.2.3
	example.com/thismodule v1.2.4

	example.com/excludedmodule v1.2.3
	example.com/excludedmodule v1.2.4

)

require (
	github.com/mattn/go-colorable v0.1.13 // indirect
	github.com/mattn/go-isatty v0.0.20 // indirect
	golang.org/x/sys v0.12.0 // indirect
	example.com/othermodule v1.2.3 // indirect
)

replace example.com/replacedmodule => ../replacedmodule

replace example.com/othermodule v1.2.3 => example.com/newmodule v3.2.1

replace (
	example.com/othermodule v1.2.4 => example.com/newmodule v3.2.2
	example.com/othermodule v1.2.5 => example.com/newmodule v3.2.3
)

exclude example.com/thismodule v1.2.3

exclude example.com/thismodule v1.2.4

exclude (
	example.com/excludedmodule v1.2.3
	example.com/excludedmodule v1.2.4
)
