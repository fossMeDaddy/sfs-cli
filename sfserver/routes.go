package sfserver

import (
	"net/http"

	"github.com/fossMeDaddy/sfs-cli/sfserver/handlers"
	"github.com/fossMeDaddy/sfs-cli/sfserver/middlewares"
	"github.com/gorilla/mux"
)

func DefineFsOpsRoutes(app *mux.Router) {
	router := app.PathPrefix("/fs").Subrouter()

	router.Use(middlewares.CheckApiKey)
	router.Use(middlewares.LoadDirTree)

	router.HandleFunc("/accesstoken", handlers.HandlePostAccessToken).Methods(http.MethodPost)
	router.HandleFunc("/tree", handlers.HandleGetTree).Methods(http.MethodGet)
	router.HandleFunc("/mkdir", handlers.HandlePostMkdir).Methods(http.MethodPost)
	router.HandleFunc("/rmdir", handlers.HandlePostRmdir).Methods(http.MethodPost)
	router.HandleFunc("/mvdir", handlers.HandlePostMvDir).Methods(http.MethodPost)
}

func DefineStorageOpsRoutes(app *mux.Router) {
	// storage handlers defined here
}
