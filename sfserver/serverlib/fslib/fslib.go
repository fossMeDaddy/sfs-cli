package fslib

import (
	"fmt"
	"slices"
	"strings"

	"github.com/google/uuid"
)

var (
	ErrDirNotFound error = fmt.Errorf("dirtree path not found")
)

type FsDirTree struct {
	Id       string
	SubPath  string
	Children []*FsDirTree
}

func NewFsDirTree() *FsDirTree {
	return &FsDirTree{
		Id: uuid.NewString(),
	}
}

func binarySearchSubPathInChildren(dirTreeChildren []*FsDirTree, subPath string) (int, bool) {
	return slices.BinarySearchFunc(dirTreeChildren, subPath, func(elem *FsDirTree, target string) int {
		if elem.SubPath == target {
			return 0
		}
		if elem.SubPath < target {
			return -1
		}
		return 1
	})
}

// will create subsequent directories if they don't exist, running again on existing paths doesn't have any effect
func (dirtree *FsDirTree) Mkdir(absolutePath string) *FsDirTree {
	currentDir := dirtree
	pathSegments := strings.Split(absolutePath, "/")
	if absolutePath != "/" {
		for _, pathSegment := range pathSegments {
			findI, found := binarySearchSubPathInChildren(currentDir.Children, pathSegment)

			var nextDir *FsDirTree
			if found {
				nextDir = currentDir.Children[findI]
			} else {
				nextDir = &FsDirTree{
					Id:      uuid.NewString(),
					SubPath: pathSegment,
				}

				currentDir.Children = slices.Insert(currentDir.Children, findI, nextDir)
			}

			currentDir = nextDir
		}
	}

	return currentDir
}

// removes a directory if exists, else returns an error. returns the removed directory if it was found & removed.
func (dirtree *FsDirTree) Rmdir(absolutePath string) (*FsDirTree, error) {
	if absolutePath == "/" {
		return nil, fmt.Errorf("can't remove the root path")
	}

	pathSegments := strings.Split(absolutePath, "/")
	currentDir := dirtree
	for i, pathSegment := range pathSegments {
		findI, found := binarySearchSubPathInChildren(currentDir.Children, pathSegment)
		if !found {
			return currentDir, ErrDirNotFound
		}

		nextDir := currentDir.Children[findI]

		if i == len(pathSegments)-1 {
			afterFindISlice := []*FsDirTree{}
			if findI < len(currentDir.Children)-1 {
				afterFindISlice = currentDir.Children[findI+1:]
			}

			currentDir.Children = slices.Concat(currentDir.Children[:findI], afterFindISlice)
		}

		currentDir = nextDir
	}

	return currentDir, nil
}

func (dirtree *FsDirTree) GetSubTree(absolutePath string) (*FsDirTree, error) {
	pathSegments := strings.Split(absolutePath, "/")
	currentDir := dirtree
	for _, pathSegment := range pathSegments {
		findI, found := binarySearchSubPathInChildren(currentDir.Children, pathSegment)
		if !found {
			return currentDir, ErrDirNotFound
		}

		currentDir = currentDir.Children[findI]
	}

	return currentDir, nil
}

func (dirtree *FsDirTree) Mvdir(oldAbsolutePath, newAbsolutePath string) error {
	rmDirTree, rmErr := dirtree.Rmdir(oldAbsolutePath)
	if rmErr != nil {
		return rmErr
	}

	pathSegments := strings.Split(newAbsolutePath, "/")
	currentDir := dirtree
	if newAbsolutePath != "/" {
		for i, pathSegment := range pathSegments {
			isLastSubPath := i == len(pathSegments)-1

			findI, found := binarySearchSubPathInChildren(currentDir.Children, pathSegment)
			if !found {
				if isLastSubPath {
					rmDirTree.SubPath = pathSegment
				} else {
					// revert changes
					dir := dirtree.Mkdir(oldAbsolutePath)
					*dir = *rmDirTree

					return ErrDirNotFound
				}

				break
			}

			currentDir = currentDir.Children[findI]
		}
	}

	insertI, _ := binarySearchSubPathInChildren(currentDir.Children, rmDirTree.SubPath)
	currentDir.Children = slices.Insert(currentDir.Children, insertI, rmDirTree)

	return nil
}

func (dirtree *FsDirTree) Walk(fn func(dir *FsDirTree)) {
	for _, child := range dirtree.Children {
		fn(child)
		child.Walk(fn)
	}
}
