package constants

const (
	ConfigPortKey           = "port"
	ConfigValkeyUriKey      = "valkey_uri"
	ConfigDatabaseUriKey    = "database_uri"
	ConfigDatabaseDriverKey = "database_driver"
	ConfigJwtSecretKey      = "jwt_secret"
)

const (
	DbDriverSqlite = "SQLITE"
	DbDriverPg     = "PG"
	DbDriverMySql  = "MYSQL"
)

var SupportedDrivers = []string{DbDriverSqlite, DbDriverPg, DbDriverMySql}
