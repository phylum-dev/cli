complete -c phylum -n "__fish_use_subcommand" -s c -l config -d 'Sets a custom config file' -r
complete -c phylum -n "__fish_use_subcommand" -s t -l timeout -d 'Set the timeout (in seconds) for requests to the Phylum api' -r
complete -c phylum -n "__fish_use_subcommand" -s h -l help -d 'Print help information'
complete -c phylum -n "__fish_use_subcommand" -s V -l version -d 'Print version information'
complete -c phylum -n "__fish_use_subcommand" -f -a "update" -d 'Check for a new release of the Phylum CLI tool and update if one exists'
complete -c phylum -n "__fish_use_subcommand" -f -a "history" -d 'Return information about historical scans'
complete -c phylum -n "__fish_use_subcommand" -f -a "projects" -d 'Create, list, link and set thresholds for projects'
complete -c phylum -n "__fish_use_subcommand" -f -a "package" -d 'Retrieve the details of a specific packge'
complete -c phylum -n "__fish_use_subcommand" -f -a "auth" -d 'Manage authentication, registration, and API keys'
complete -c phylum -n "__fish_use_subcommand" -f -a "ping" -d 'Ping the remote system to verify it is available'
complete -c phylum -n "__fish_use_subcommand" -f -a "analyze" -d 'Submit a request for analysis to the processing system'
complete -c phylum -n "__fish_use_subcommand" -f -a "batch" -d 'Submits a batch of requests to the processing system'
complete -c phylum -n "__fish_use_subcommand" -f -a "version" -d 'Display application version'
complete -c phylum -n "__fish_use_subcommand" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c phylum -n "__fish_seen_subcommand_from update" -s p -l prerelease -d 'Update to the latest prerelease (vs. stable, default: false)'
complete -c phylum -n "__fish_seen_subcommand_from update" -s h -l help -d 'Print help information'
complete -c phylum -n "__fish_seen_subcommand_from update" -s V -l version -d 'Print version information'
complete -c phylum -n "__fish_seen_subcommand_from history" -l filter -d 'Provide a filter used to limit the issues displayed

EXAMPLES
# Show only issues with severity of at least \'high\'
	--filter=high

# Show issues with severity of \'critical\' in the \'author\' and \'engineering\' domains
	--filter=crit,aut,eng
' -r
complete -c phylum -n "__fish_seen_subcommand_from history" -d 'The job id to query (or `current` for the most recent job)'
complete -c phylum -n "__fish_seen_subcommand_from history" -s v -l verbose -d 'Increase verbosity of api response.'
complete -c phylum -n "__fish_seen_subcommand_from history" -s j -l json -d 'Produce output in json format (default: false)'
complete -c phylum -n "__fish_seen_subcommand_from history" -s h -l help -d 'Print help information'
complete -c phylum -n "__fish_seen_subcommand_from history" -s V -l version -d 'Print version information'
complete -c phylum -n "__fish_seen_subcommand_from history" -f -a "project" -d 'Shows a list of projects associated with the user'
complete -c phylum -n "__fish_seen_subcommand_from project" -l help -d 'Print help information'
complete -c phylum -n "__fish_seen_subcommand_from project" -l version -d 'Print version information'
complete -c phylum -n "__fish_seen_subcommand_from projects" -s h -l help -d 'Print help information'
complete -c phylum -n "__fish_seen_subcommand_from projects" -s V -l version -d 'Print version information'
complete -c phylum -n "__fish_seen_subcommand_from projects" -f -a "create" -d 'Create a new project'
complete -c phylum -n "__fish_seen_subcommand_from projects" -f -a "list" -d 'List all existing projects'
complete -c phylum -n "__fish_seen_subcommand_from projects" -f -a "link" -d 'Link a repository to a project'
complete -c phylum -n "__fish_seen_subcommand_from projects" -f -a "set-thresholds" -d 'Set risk domain thresholds for a projects'
complete -c phylum -n "__fish_seen_subcommand_from create" -l help -d 'Print help information'
complete -c phylum -n "__fish_seen_subcommand_from create" -l version -d 'Print version information'
complete -c phylum -n "__fish_seen_subcommand_from list" -l help -d 'Print help information'
complete -c phylum -n "__fish_seen_subcommand_from list" -l version -d 'Print version information'
complete -c phylum -n "__fish_seen_subcommand_from link" -l help -d 'Print help information'
complete -c phylum -n "__fish_seen_subcommand_from link" -l version -d 'Print version information'
complete -c phylum -n "__fish_seen_subcommand_from set-thresholds" -l help -d 'Print help information'
complete -c phylum -n "__fish_seen_subcommand_from set-thresholds" -l version -d 'Print version information'
complete -c phylum -n "__fish_seen_subcommand_from package" -s t -l package-type -d 'The type of the package ("npm", "ruby", "pypi", etc.)' -r
complete -c phylum -n "__fish_seen_subcommand_from package" -s j -l json -d 'Produce output in json format (default: false)'
complete -c phylum -n "__fish_seen_subcommand_from package" -s h -l help -d 'Print help information'
complete -c phylum -n "__fish_seen_subcommand_from package" -s V -l version -d 'Print version information'
complete -c phylum -n "__fish_seen_subcommand_from auth" -s h -l help -d 'Print help information'
complete -c phylum -n "__fish_seen_subcommand_from auth" -s V -l version -d 'Print version information'
complete -c phylum -n "__fish_seen_subcommand_from auth" -f -a "register" -d 'Register a new account'
complete -c phylum -n "__fish_seen_subcommand_from auth" -f -a "login" -d 'Login to an existing account'
complete -c phylum -n "__fish_seen_subcommand_from auth" -f -a "status" -d 'Return the current authentication status'
complete -c phylum -n "__fish_seen_subcommand_from register" -l help -d 'Print help information'
complete -c phylum -n "__fish_seen_subcommand_from register" -l version -d 'Print version information'
complete -c phylum -n "__fish_seen_subcommand_from login" -l help -d 'Print help information'
complete -c phylum -n "__fish_seen_subcommand_from login" -l version -d 'Print version information'
complete -c phylum -n "__fish_seen_subcommand_from status" -l help -d 'Print help information'
complete -c phylum -n "__fish_seen_subcommand_from status" -l version -d 'Print version information'
complete -c phylum -n "__fish_seen_subcommand_from ping" -s h -l help -d 'Print help information'
complete -c phylum -n "__fish_seen_subcommand_from ping" -s V -l version -d 'Print version information'
complete -c phylum -n "__fish_seen_subcommand_from analyze" -s l -r
complete -c phylum -n "__fish_seen_subcommand_from analyze" -l filter -d 'Provide a filter used to limit the issues displayed

EXAMPLES
# Show only issues with severity of at least \'high\'
	--filter=high

# Show issues with severity of \'critical\' in the \'author\' and \'engineering\' domains
	--filter=crit,aut,eng
' -r
complete -c phylum -n "__fish_seen_subcommand_from analyze" -d 'The package lock file to submit.'
complete -c phylum -n "__fish_seen_subcommand_from analyze" -s v -l verbose -d 'Increase verbosity of api response.'
complete -c phylum -n "__fish_seen_subcommand_from analyze" -s j -l json -d 'Produce output in json format (default: false)'
complete -c phylum -n "__fish_seen_subcommand_from analyze" -s F -d 'Force re-processing of packages (even if they already exist in the system)'
complete -c phylum -n "__fish_seen_subcommand_from analyze" -s h -l help -d 'Print help information'
complete -c phylum -n "__fish_seen_subcommand_from analyze" -s V -l version -d 'Print version information'
complete -c phylum -n "__fish_seen_subcommand_from batch" -s f -d 'File (or piped stdin) containing the list of packages (format `<name>:<version>`)' -r
complete -c phylum -n "__fish_seen_subcommand_from batch" -s t -d 'Package type (`npm`, `ruby`, etc)' -r
complete -c phylum -n "__fish_seen_subcommand_from batch" -s l -r
complete -c phylum -n "__fish_seen_subcommand_from batch" -s F -d 'Force re-processing of packages (even if they already exist in the system)'
complete -c phylum -n "__fish_seen_subcommand_from batch" -s L
complete -c phylum -n "__fish_seen_subcommand_from batch" -s h -l help -d 'Print help information'
complete -c phylum -n "__fish_seen_subcommand_from batch" -s V -l version -d 'Print version information'
complete -c phylum -n "__fish_seen_subcommand_from version" -s h -l help -d 'Print help information'
complete -c phylum -n "__fish_seen_subcommand_from version" -s V -l version -d 'Print version information'
complete -c phylum -n "__fish_seen_subcommand_from help" -s h -l help -d 'Print help information'
complete -c phylum -n "__fish_seen_subcommand_from help" -s V -l version -d 'Print version information'
