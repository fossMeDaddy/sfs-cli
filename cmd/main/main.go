package main

import (
	"os"

	"github.com/fossMeDaddy/sfs-cli/cli/deployCmd"
	"github.com/fossMeDaddy/sfs-cli/cli/serveCmd"
	"github.com/spf13/cobra"
)

func main() {
	rootCmd := &cobra.Command{
		Use:   "sfs",
		Short: "Simple Fucking Storage.",
	}

	rootCmd.AddCommand(serveCmd.Init())
	rootCmd.AddCommand(deployCmd.Init())

	if err := rootCmd.Execute(); err != nil {
		rootCmd.PrintErrln("unexpected error occured")
		os.Exit(1)
	}
}
