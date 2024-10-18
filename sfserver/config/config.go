package config

import (
	"fmt"

	"github.com/fossMeDaddy/sfs-cli/constants"
	"github.com/joho/godotenv"
	"github.com/spf13/viper"
)

func SetDefaultValues() {
	viper.SetDefault(constants.ConfigPortKey, 30035)

	viper.SetDefault(constants.ConfigDatabaseUriKey, "data.db")
	viper.SetDefault(constants.ConfigDatabaseDriverKey, "SQLITE")
}

func LoadConfigFromEnv(verboseOutput bool) {
	godotenv.Load(".env")

	viper.BindEnv(constants.ConfigPortKey)
	viper.BindEnv(constants.ConfigJwtSecretKey)

	viper.BindEnv(constants.ConfigDatabaseUriKey)
	viper.BindEnv(constants.ConfigDatabaseDriverKey)

	viper.BindEnv(constants.ConfigValkeyUriKey)

	if verboseOutput {
		fmt.Println("Config options:")

		if viper.GetString(constants.ConfigValkeyUriKey) == "" {
			fmt.Println(fmt.Sprintf("%s: not provided, using in-memory cache (however, it is advised to use a valkey instance)", constants.ConfigValkeyUriKey))
		} else {
			fmt.Println(fmt.Sprintf("%s: %s", constants.ConfigValkeyUriKey, viper.GetString(constants.ConfigValkeyUriKey)))
		}
		fmt.Println(fmt.Sprintf("%s: %s", constants.ConfigPortKey, viper.GetString(constants.ConfigPortKey)))
		fmt.Println(fmt.Sprintf("%s: %s", constants.ConfigDatabaseDriverKey, viper.GetString(constants.ConfigDatabaseDriverKey)))
		fmt.Println(fmt.Sprintf("%s: %s", constants.ConfigDatabaseUriKey, viper.GetString(constants.ConfigDatabaseUriKey)))
	}
}
