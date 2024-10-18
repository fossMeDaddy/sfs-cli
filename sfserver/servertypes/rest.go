package servertypes

type Response struct {
	Message string
	Data    interface{}
}

type ValidationError struct {
	FailedField string
	Tag         string
	Value       interface{}
}
