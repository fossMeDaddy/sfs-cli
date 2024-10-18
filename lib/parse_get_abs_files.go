package lib

import (
	"fmt"
	"io/fs"
	"os"
	"path"
	"path/filepath"
	"regexp"
	"slices"
	"strings"
)

func parseAbsPathPattern(absPatt string) ([]string, error) {
	absPatt = strings.TrimRight(absPatt, string(filepath.Separator))

	paths := []string{}

	refWd := ""
	pathlist := strings.Split(absPatt, string(filepath.Separator))
	for _, path := range pathlist {
		if strings.Contains(path, "*") || (strings.Contains(path, "{") && strings.Contains(path, "}")) {
			break
		}

		refWd += path + string(filepath.Separator)
	}
	refWd = strings.TrimRight(refWd, string(filepath.Separator))

	if refWd == absPatt {
		stat, statErr := os.Stat(refWd)
		if statErr != nil {
			return paths, statErr
		}

		if !stat.IsDir() {
			paths = append(paths, refWd)
			return paths, nil
		}

		dirEntries, err := os.ReadDir(refWd)
		if err != nil {
			return paths, err
		}

		for _, dirEntry := range dirEntries {
			if !dirEntry.IsDir() {
				paths = append(paths, path.Join(refWd, dirEntry.Name()))
			}
		}

		return paths, nil
	}

	absPatt = strings.ReplaceAll(absPatt, string(filepath.Separator), fmt.Sprintf(`\%s`, string(filepath.Separator)))
	absPatt = strings.ReplaceAll(absPatt, "(", `\(`)
	absPatt = strings.ReplaceAll(absPatt, ")", `\)`)
	absPatt = strings.ReplaceAll(absPatt, "[", `\[`)
	absPatt = strings.ReplaceAll(absPatt, "]", `\]`)
	absPatt = strings.ReplaceAll(absPatt, "-", `\-`)
	absPatt = strings.ReplaceAll(absPatt, ".", `\.`)

	absPatt = strings.ReplaceAll(absPatt, "**", `.+`)
	absPatt = strings.ReplaceAll(absPatt, "*", fmt.Sprintf(`[^\%s]*`, string(filepath.Separator)))

	absPatt = "^" + absPatt + "$"

	re := regexp.MustCompile(`\{([\w\|]+)\}`)
	absPatt = re.ReplaceAllString(absPatt, "($1)")

	// TODO: remove this line after some testing
	fmt.Println("FILE PATTERN CONVERTED TO REGEX:", absPatt)

	absPattRe, pathReErr := regexp.Compile(absPatt)
	if pathReErr != nil {
		return paths, pathReErr
	}

	if len(refWd) == 0 {
		return paths, fmt.Errorf("invalid path received!")
	}

	if err := filepath.Walk(refWd, func(path string, info fs.FileInfo, err error) error {
		if absPattRe.MatchString(path) && !info.IsDir() {
			paths = append(paths, path)
		}

		return nil
	}); err != nil {
		return paths, err
	}

	return paths, nil
}

// takes in file paths like "./bin/*.exe" and gives out absolute paths to the files matching
//
// supported patterns
//
//	'*': matchall (matches all files & folders in a directory)
//	'**': matchall (matches all files & folders in subsequent directories)
//	'{X|Y|Z}': X or Y or Z
func ParseGetAbsFiles(pathPatterns []string) ([]string, error) {
	absPaths := []string{}

	for _, patt := range pathPatterns {
		absPatt, err := filepath.Abs(patt)
		if err != nil {
			return absPaths, err
		}

		paths, pattParseErr := parseAbsPathPattern(absPatt)
		if pattParseErr != nil {
			return absPaths, pattParseErr
		}

		absPaths = slices.Concat(absPaths, paths)
	}

	slices.Sort(absPaths)
	return absPaths, nil
}
