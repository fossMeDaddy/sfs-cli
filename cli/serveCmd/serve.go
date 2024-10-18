package serveCmd

import (
	"encoding/json"
	"fmt"
	"math/rand"
	"net/http"
	"net/url"
	"slices"
	"sync"
	"time"

	"github.com/fossMeDaddy/sfs-cli/lib"
	"github.com/fossMeDaddy/sfs-cli/sfserver/servertypes"
	"github.com/gorilla/mux"
	"github.com/spf13/cobra"
)

var (
	allowAnyAddrFlag  bool
	excludedFilesFlag []string
)

var cmdExamples = `
	# mind the "quotes" !!

	serve "~/img/project_logo.png" "~/build/project.exe" "./bin/*"
	serve "build/*.html"
	serve "img/*.{jpeg,png,jpg,webp}"
	serve "bin/build_output_*"
	serve "important_doc.{pdf,xlsx}"
	serve "./projects/sfs/**" --exclude "./projects/sfs/**/*.env"
`

var cmd = &cobra.Command{
	Use:     "serve",
	Short:   "serve files over HTTP in your LAN",
	Example: cmdExamples,
	Run:     run,
	Args:    cobra.MinimumNArgs(1),
}

func run(cmd *cobra.Command, args []string) {
	var wg sync.WaitGroup

	ipv4, err := lib.GetIpv4()
	if err != nil {
		cmd.PrintErrln("WARNING: error occured while fetching your local ipv4 address")
		cmd.PrintErrln(err)
		cmd.PrintErrln("setting to 0.0.0.0 to allow any device on your network to connect to you!")

		ipv4 = "0.0.0.0"
	}

	if allowAnyAddrFlag {
		ipv4 = "0.0.0.0"
	}

	var exFiles []string
	var exFilesErr error
	wg.Add(1)
	go (func() {
		defer wg.Done()
		exFiles, exFilesErr = lib.ParseGetAbsFiles(excludedFilesFlag)
	})()

	files, parseErr := lib.ParseGetAbsFiles(args)
	if parseErr != nil {
		cmd.PrintErrln(parseErr)
		return
	}
	wg.Wait()

	if exFilesErr != nil {
		cmd.PrintErrln(exFilesErr)
		return
	}

	if len(files) == 0 {
		cmd.PrintErrln("no files found to serve")
		return
	}

	fc := 0
	for i, file := range files {
		if _, isExcluded := slices.BinarySearch(exFiles, file); isExcluded {
			files[i] = ""
			continue
		}

		fc++
	}

	cmd.Println(fmt.Sprintf("serving %d files...", fc))

	app := mux.NewRouter()

	app.Path("/").Methods(http.MethodGet).HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "text/html")

		resHTML := "<ul>"
		for _, file := range files {
			if file == "" {
				continue
			}
			urlEncFile := url.QueryEscape(file)
			resHTML += fmt.Sprintf(`<li><a href="/file?path=%s">%s</a></li><br>`, urlEncFile, file)
		}
		resHTML += "</ul>"

		w.Write([]byte(resHTML))
	})

	app.Path("/file").Queries("path", "{.+}").Methods(http.MethodGet).HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		fp, err := url.QueryUnescape(r.URL.Query().Get("path"))
		if err != nil {
			w.WriteHeader(http.StatusBadRequest)
			json.NewEncoder(w).Encode(servertypes.Response{
				Message: err.Error(),
			})
			return
		}

		_, found := slices.BinarySearch(files, fp)
		if !found || fp == "" {
			w.WriteHeader(http.StatusNotFound)
			return
		}

		http.ServeFile(w, r, fp)
		return
	})

	port := uint16(rand.Float32()*float32(55_000) + 10_000) // generate random port numbers in range [10_000, 65_000)
	addr := fmt.Sprintf("%s:%d", ipv4, port)
	server := http.Server{
		Addr:        addr,
		Handler:     app,
		ReadTimeout: 2 * time.Second,
	}

	cmd.Println()
	cmd.Println(fmt.Sprintf("starting server at: http://%s", addr))

	if err := server.ListenAndServe(); err != nil {
		cmd.PrintErrln("unexpected error occured while starting server")
		cmd.PrintErrln(err)
		return
	}
}

func Init() *cobra.Command {
	cmd.Flags().BoolVarP(&allowAnyAddrFlag, "any-addr", "a", false, "allow server to receive requests from any address '0.0.0.0'")
	cmd.Flags().StringSliceVarP(&excludedFilesFlag, "excluded", "e", []string{}, "provide a list of comma-separated, quotes-wrapped patterns to exclude from serving")

	return cmd
}
