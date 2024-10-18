package handlers

import (
	"encoding/json"
	"fmt"
	"net/http"

	"github.com/fossMeDaddy/sfs-cli/sfserver/servertypes"
)

func CatchAll(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusNotFound)
	json.NewEncoder(w).Encode(servertypes.Response{
		Message: fmt.Sprintf("route '%s' not found", r.URL.Path),
	})
	return
}
