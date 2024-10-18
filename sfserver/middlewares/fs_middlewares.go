package middlewares

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"

	"github.com/fossMeDaddy/sfs-cli/sfserver/db/models"
	"github.com/fossMeDaddy/sfs-cli/sfserver/serverconst"
	"github.com/fossMeDaddy/sfs-cli/sfserver/serverglobals"
	"github.com/fossMeDaddy/sfs-cli/sfserver/serverlib/fslib"
	"github.com/fossMeDaddy/sfs-cli/sfserver/servertypes"
	"gorm.io/gorm"
)

func CheckApiKey(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		authHeader := r.Header.Get(serverconst.ApiKeyHeaderName)
		if len(authHeader) == 0 {
			w.WriteHeader(http.StatusUnauthorized)
			json.NewEncoder(w).Encode(servertypes.Response{
				Message: fmt.Sprintf("header '%s' is required", serverconst.ApiKeyHeaderName),
			})
			return
		}

		apiKey := models.ApiKey{}
		tx := serverglobals.DB.Where(&models.ApiKey{Key: authHeader}).First(&apiKey)
		if tx.Error != nil {
			w.WriteHeader(http.StatusUnauthorized)
			json.NewEncoder(w).Encode(servertypes.Response{
				Message: "invalid api key",
			})
			return
		}

		newCtx := context.WithValue(r.Context(), serverconst.LocalsApiKeyKey, &apiKey)
		next.ServeHTTP(w, r.WithContext(newCtx))
	})
}

// call this after apikey is present in locals
func LoadDirTree(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		apiKey := r.Context().Value(serverconst.LocalsApiKeyKey).(*models.ApiKey)
		if apiKey == nil {
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(servertypes.Response{
				Message: "api key nil, author skill issues",
			})
			return
		}

		fs := models.FileSystem{}
		if tx := serverglobals.DB.Where(&models.FileSystem{Id: apiKey.Key}).First(&fs); errors.Is(tx.Error, gorm.ErrRecordNotFound) {
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(servertypes.Response{
				Message: "author skill issues, filesystem not found",
			})
			return
		} else if tx.Error != nil {
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(servertypes.Response{
				Message: "unexpected error occured, err: " + tx.Error.Error(),
			})
			return
		}

		dirtree := fslib.FsDirTree{}
		if err := json.Unmarshal([]byte(fs.DirTreeJson), &dirtree); err != nil {
			w.WriteHeader(http.StatusInternalServerError)
			json.NewEncoder(w).Encode(servertypes.Response{
				Message: "HEAVY author skill issues, fs dir tree unmarshal error: " + err.Error(),
			})
			return
		}

		newCtx := context.WithValue(r.Context(), serverconst.LocalsDirTree, &dirtree)
		newCtx = context.WithValue(newCtx, serverconst.LocalsFileSystemKey, &fs)
		next.ServeHTTP(w, r.WithContext(newCtx))

	})
}
