package handlers

import (
	"encoding/json"
	"fmt"
	"net/http"
	"slices"
	"time"

	"github.com/fossMeDaddy/sfs-cli/lib"
	"github.com/fossMeDaddy/sfs-cli/sfserver/db/models"
	"github.com/fossMeDaddy/sfs-cli/sfserver/serverconst"
	"github.com/fossMeDaddy/sfs-cli/sfserver/serverglobals"
	"github.com/fossMeDaddy/sfs-cli/sfserver/serverlib"
	"github.com/fossMeDaddy/sfs-cli/sfserver/serverlib/fslib"
	"github.com/fossMeDaddy/sfs-cli/sfserver/servertypes"
)

// fs mkdir, [GET] tree --metadata (dirpath), rm (empty dirpaths & filepaths), rmrf (dirpaths), touch (filepath)

func updateFsDirTree(filesystem *models.FileSystem, dirtree *fslib.FsDirTree) error {
	jsonB, jsonErr := json.Marshal(dirtree)
	if jsonErr != nil {
		fmt.Println(filesystem)
		fmt.Println(dirtree)
		panic("json encoding error holy fuck")
	}
	if tx := serverglobals.DB.Model(&models.FileSystem{}).Where(
		&models.FileSystem{Id: filesystem.Id},
	).Update("dir_tree_json", string(jsonB)); tx.Error != nil {
		return tx.Error
	}

	return nil
}

func HandlePostMvDir(w http.ResponseWriter, r *http.Request) {
	encoder := json.NewEncoder(w)

	filesystem, ok := r.Context().Value(serverconst.LocalsFileSystemKey).(*models.FileSystem)
	if !ok {
		w.WriteHeader(http.StatusInternalServerError)
		encoder.Encode(servertypes.Response{
			Message: "filesystem nil, server endpoint author skill issues",
		})
		return
	}

	dirtree, ok := r.Context().Value(serverconst.LocalsDirTree).(*fslib.FsDirTree)
	if !ok {
		w.WriteHeader(http.StatusInternalServerError)
		encoder.Encode(servertypes.Response{
			Message: "dirtree nil, server endpoint author skill issues, HEAVY SKILL ISSUES",
		})
		return
	}

	reqBody := struct {
		OldPath string
		NewPath string
	}{}
	if err := serverlib.ParseValidateJsonBody(w, r, &reqBody); err != nil {
		fmt.Println(err)
		return
	}
	if path, err := lib.ParseAbsoluteFsPath(reqBody.OldPath); err != nil {
		w.WriteHeader(http.StatusBadRequest)
		encoder.Encode(servertypes.Response{
			Message: err.Error(),
		})
		return
	} else {
		reqBody.OldPath = path
	}
	if path, err := lib.ParseAbsoluteFsPath(reqBody.NewPath); err != nil {
		w.WriteHeader(http.StatusBadRequest)
		encoder.Encode(servertypes.Response{
			Message: err.Error(),
		})
		return
	} else {
		reqBody.NewPath = path
	}

	if err := dirtree.Mvdir(reqBody.OldPath, reqBody.NewPath); err != nil {
		w.WriteHeader(http.StatusBadRequest)
		encoder.Encode(servertypes.Response{
			Message: err.Error(),
		})
		return
	}

	if err := updateFsDirTree(filesystem, dirtree); err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		encoder.Encode(servertypes.Response{
			Message: "unexpected error occured while updating dirtree " + err.Error(),
		})
		return
	}

	encoder.Encode(servertypes.Response{
		Data: dirtree,
	})
	return
}

func HandlePostRmdir(w http.ResponseWriter, r *http.Request) {
	encoder := json.NewEncoder(w)

	filesystem, ok := r.Context().Value(serverconst.LocalsFileSystemKey).(*models.FileSystem)
	if !ok {
		w.WriteHeader(http.StatusInternalServerError)
		encoder.Encode(servertypes.Response{
			Message: "filesystem nil, server endpoint author skill issues",
		})
		return
	}

	dirtree, ok := r.Context().Value(serverconst.LocalsDirTree).(*fslib.FsDirTree)
	if !ok {
		w.WriteHeader(http.StatusInternalServerError)
		encoder.Encode(servertypes.Response{
			Message: "dirtree nil, server endpoint author skill issues, HEAVY SKILL ISSUES",
		})
		return
	}

	reqBody := struct {
		Path string
		Opts []string
	}{}
	if err := serverlib.ParseValidateJsonBody(w, r, &reqBody); err != nil {
		fmt.Println(err)
		return
	}
	if path, err := lib.ParseAbsoluteFsPath(reqBody.Path); err != nil {
		w.WriteHeader(http.StatusBadRequest)
		encoder.Encode(servertypes.Response{
			Message: err.Error(),
		})
		return
	} else if path == "/" {
		w.WriteHeader(http.StatusBadRequest)
		encoder.Encode(servertypes.Response{
			Message: "cannot remove '/' directory, it's the root path",
		})
	} else {
		reqBody.Path = path
	}

	currentDir, dirTreeErr := dirtree.GetSubTree(reqBody.Path)
	if dirTreeErr != nil {
		w.WriteHeader(http.StatusInternalServerError)
		encoder.Encode(servertypes.Response{
			Message: dirTreeErr.Error(),
		})
		return
	}

	if slices.Contains(reqBody.Opts, "force") {
		rmDirIds := []string{}
		currentDir.Walk(func(dir *fslib.FsDirTree) {
			rmDirIds = append(rmDirIds, dir.Id)
		})

		nowTime := time.Now()
		if tx := serverglobals.DB.Model(&models.FsFile{}).Where(
			map[string]interface{}{"file_system_id": filesystem.Id, "dir_id": rmDirIds},
		).Updates(&models.FsFile{DeletedAt: &nowTime}); tx.Error != nil {
			w.WriteHeader(http.StatusInternalServerError)
			encoder.Encode(servertypes.Response{
				Message: "failed deleting files in all the subdirectories " + tx.Error.Error(),
			})
			return
		}

		// TODO: create notification for all users with access control for the removed directories, files are scheduled for deletion...
	} else {
		if len(currentDir.Children) > 0 {
			w.WriteHeader(http.StatusBadRequest)
			encoder.Encode(servertypes.Response{
				Message: "directory contains subdirectories, either empty this directory or use 'force' option",
			})
			return
		}

		var nFiles int64
		if tx := serverglobals.DB.Where(&models.FsFile{FileSystemId: filesystem.Id, DirId: currentDir.Id}, "FileSystemId", "DirId").Count(&nFiles); tx.Error != nil {
			w.WriteHeader(http.StatusBadRequest)
			encoder.Encode(servertypes.Response{
				Message: "could not fetch files under the directory",
			})
			return
		}
		if nFiles > 0 {
			w.WriteHeader(http.StatusBadRequest)
			encoder.Encode(servertypes.Response{
				Message: fmt.Sprintf("can't remove '%s', dir not empty (tip: use 'files' or 'dirs' options to also remove the content in the directory)", currentDir.SubPath),
			})
			return
		}
	}

	if _, err := dirtree.Rmdir(reqBody.Path); err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		encoder.Encode(servertypes.Response{
			Message: "unexpected error occured while removing the directory " + err.Error(),
		})
		return
	}

	if err := updateFsDirTree(filesystem, dirtree); err != nil {
		w.WriteHeader(http.StatusBadRequest)
		encoder.Encode(servertypes.Response{
			Message: "unexpected error occured while updating dirtree " + err.Error(),
		})
		return
	}

	encoder.Encode(servertypes.Response{
		Data: dirtree,
	})
	return
}

func HandleGetTree(w http.ResponseWriter, r *http.Request) {
	encoder := json.NewEncoder(w)

	dirtree, ok := r.Context().Value(serverconst.LocalsDirTree).(*fslib.FsDirTree)
	if !ok {
		w.WriteHeader(http.StatusInternalServerError)
		encoder.Encode(servertypes.Response{
			Message: "dirtree nil, server endpoint author skill issues, HEAVY SKILL ISSUES",
		})
		return
	}

	encoder.Encode(servertypes.Response{
		Data: dirtree,
	})
	return
}

func HandlePostMkdir(w http.ResponseWriter, r *http.Request) {
	encoder := json.NewEncoder(w)

	filesystem, ok := r.Context().Value(serverconst.LocalsFileSystemKey).(*models.FileSystem)
	if !ok {
		w.WriteHeader(http.StatusInternalServerError)
		encoder.Encode(servertypes.Response{
			Message: "filesystem nil, server endpoint author skill issues",
		})
		return
	}

	dirtree, ok := r.Context().Value(serverconst.LocalsDirTree).(*fslib.FsDirTree)
	if !ok {
		w.WriteHeader(http.StatusInternalServerError)
		encoder.Encode(servertypes.Response{
			Message: "dirtree nil, server endpoint author skill issues, HEAVY SKILL ISSUES",
		})
		return
	}

	reqBody := struct {
		Path string
	}{}
	if err := serverlib.ParseValidateJsonBody(w, r, &reqBody); err != nil {
		fmt.Println(err)
		return
	}
	if path, err := lib.ParseAbsoluteFsPath(reqBody.Path); err != nil {
		w.WriteHeader(http.StatusBadRequest)
		encoder.Encode(servertypes.Response{
			Message: err.Error(),
		})
		return
	} else if path == "/" {
		w.WriteHeader(http.StatusBadRequest)
		encoder.Encode(servertypes.Response{
			Message: "cannot create '/' directory, already exists",
		})
	} else {
		reqBody.Path = path
	}

	dirtree.Mkdir(reqBody.Path)

	if err := updateFsDirTree(filesystem, dirtree); err != nil {
		w.WriteHeader(http.StatusBadRequest)
		encoder.Encode(servertypes.Response{
			Message: "unexpected error occured while updating dirtree " + err.Error(),
		})
		return
	}

	encoder.Encode(servertypes.Response{
		Data: dirtree,
	})
	return
}

func HandlePostAccessToken(w http.ResponseWriter, r *http.Request) {
}
