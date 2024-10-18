package lib

import (
	"fmt"
	"strings"
)

func ParseAbsoluteFsPath(path string) (string, error) {
	path = strings.Trim(path, "/")
	if len(path) == 0 {
		path = "/"
	}

	if strings.Contains(path, "//") {
		return path, fmt.Errorf("repeating slashes not allowed")
	}

	return path, nil
}
