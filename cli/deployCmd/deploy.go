package deployCmd

import (
	"github.com/fossMeDaddy/sfs-cli/sfserver"
	"github.com/spf13/cobra"
)

var cmdManual = `
To just fucking run the server with sensible defaults:
	- Database: local SQLite db
	- Ratelimiter: in-memory
	- Files storage: local disk

Execute the following command:
$ JWT_SECRET="my_deepest$%# darkest 5ecre3T" sfs deploy

Or if your kink is reading manuals, please continue reading...

Server is configurable via ENV variables:

- PORT: if not given, defaults to 30035
- JWT_SECRET: absolutely required, can be any random string
- VALKEY_URI: (uses redis's url spec, e.g. 'redis://...') used as usage counter & rate-limiter, a nice to have component but, completely optional.
- DATABASE_URI: if not given, defaults to locally created sqlite database
- DATABASE_DRIVER: if not given, defaults to "SQLITE"
	POSSIBLE VALUES: "SQLITE" | "PG" | "MYSQL"

- STORAGE_STRATEGY: if not given, defaults to local disk of the machine. Defines how to handle uploaded files.
	Possible values: "LOCAL" | "S3" | "R2"

	- LOCAL: rawdawg them files right in the disk.
	- S3: send uploaded files over to AWS's Simple (really?!) Storage Service (S3)
	- R2: send uploaded files over to Cloudflare's R2

	Note: "S3" & "R2" strategies require credentials with correct permissions to operate.

# If STORAGE_STRATEGY was chosen to be "S3" or "R2"
- ACCESS_KEY_ID
- SECRET_ACCESS_KEY
- BUCKET_NAME
- REGION
- R2_ACCOUNT_ID (only when using R2, not required with S3)

NOTE:
RTF(riendly)M for configuring S3 or R2 with these credentials or give the sfs cli codebase a visit on github to know more about how we're configuring these things.
feel free to raise an issue if you feel like we're doing something wrong: https://github.com/fossMeDaddy/sfs-cli
`

var cmd *cobra.Command = &cobra.Command{
	Use:   "deploy",
	Short: "deploy your own SFS server, run 'help deploy' to know more about configuring this server.",
	Long:  cmdManual,
	Run:   run,
}

func run(cmd *cobra.Command, args []string) {
	if err := sfserver.StartServer(true); err != nil {
		cmd.PrintErrln("error occured while starting the server")
		cmd.PrintErrln(err)
		return
	}
}

func Init() *cobra.Command {
	// flags here

	return cmd
}
