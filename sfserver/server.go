package sfserver

import (
	"fmt"
	"net/http"
	"time"

	"github.com/gorilla/mux"
	"github.com/spf13/viper"

	"github.com/fossMeDaddy/sfs-cli/constants"
	"github.com/fossMeDaddy/sfs-cli/lib"
	"github.com/fossMeDaddy/sfs-cli/sfserver/config"
	"github.com/fossMeDaddy/sfs-cli/sfserver/db"
	"github.com/fossMeDaddy/sfs-cli/sfserver/handlers"
	"github.com/fossMeDaddy/sfs-cli/sfserver/middlewares"
	"github.com/fossMeDaddy/sfs-cli/sfserver/serverglobals"
	"github.com/fossMeDaddy/sfs-cli/sfserver/serverlib/cachelib"
)

func StartServer(verboseOutput bool) error {
	// init viper config
	config.SetDefaultValues()
	config.LoadConfigFromEnv(verboseOutput)

	ipv4, _ := lib.GetIpv4()
	if verboseOutput {
		fmt.Println("LOCAL IP:", ipv4)
	}

	if db, err := db.ConnectGormDb(
		viper.GetString(constants.ConfigDatabaseDriverKey),
		viper.GetString(constants.ConfigDatabaseUriKey),
		true,
	); err != nil {
		return err
	} else {
		serverglobals.DB = db
	}

	if client, err := cachelib.NewCacheClient(
		viper.GetString(constants.ConfigValkeyUriKey),
	); err != nil {
		return err
	} else {
		serverglobals.CacheClient = client
	}

	app := mux.NewRouter()
	app.Use(middlewares.SetContentTypeJson)

	DefineFsOpsRoutes(app)
	DefineStorageOpsRoutes(app)

	app.PathPrefix("/").HandlerFunc(handlers.CatchAll)

	server := http.Server{
		Addr:        fmt.Sprintf(":%s", viper.GetString(constants.ConfigPortKey)),
		Handler:     app,
		ReadTimeout: 10 * time.Second,
	}

	if verboseOutput {
		fmt.Println()
		fmt.Println(fmt.Sprintf("server started listening at: 0.0.0.0:%s", viper.GetString(constants.ConfigPortKey)))
	}
	return server.ListenAndServe()
}
