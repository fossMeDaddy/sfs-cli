package serverlib

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"

	"github.com/fossMeDaddy/sfs-cli/sfserver/servertypes"
	"github.com/go-playground/validator/v10"
)

var (
	validate = validator.New()
)

func registerAbsoluteFsPathValidator(fl validator.FieldLevel) bool {
	field := fl.Field().String()

	if !strings.HasPrefix(field, "/") {
		return false
	}

	return true
}

func ValidateData(data interface{}) []servertypes.ValidationError {
	validationErrors := []servertypes.ValidationError{}

	validate.RegisterValidation("absolute_fs_path", registerAbsoluteFsPathValidator)

	errs := validate.Struct(data)
	if errs != nil {
		for _, err := range errs.(validator.ValidationErrors) {
			validationErr := servertypes.ValidationError{
				FailedField: err.Field(),
				Tag:         err.Tag(),
				Value:       err.Value(),
			}

			validationErrors = append(validationErrors, validationErr)
		}
	}

	return validationErrors
}

// reads & unmarshals the request body into "reqBodyStructPtr" pointer.
// writes header & response for any errors, returns error to signify that the response was written
func ParseValidateJsonBody(w http.ResponseWriter, r *http.Request, reqBodyStructPtr interface{}) error {
	encoder := json.NewEncoder(w)

	reqBodyB, bodyErr := io.ReadAll(r.Body)
	if bodyErr != nil {
		w.WriteHeader(http.StatusBadRequest)
		encoder.Encode(servertypes.Response{
			Message: bodyErr.Error(),
		})
		return fmt.Errorf("request body read error: %s", bodyErr.Error())
	}

	if err := json.Unmarshal(reqBodyB, reqBodyStructPtr); err != nil {
		w.WriteHeader(http.StatusBadRequest)
		encoder.Encode(servertypes.Response{
			Message: "expected request body format to be 'json' error occured: " + err.Error(),
		})
		return fmt.Errorf("json unmarshal error: %s", err.Error())
	}

	if valErrs := ValidateData(reqBodyStructPtr); len(valErrs) > 0 {
		w.WriteHeader(http.StatusBadRequest)
		encoder.Encode(servertypes.Response{
			Message: "validation error",
			Data:    valErrs,
		})
		return fmt.Errorf("validation errors found")
	}

	return nil
}
