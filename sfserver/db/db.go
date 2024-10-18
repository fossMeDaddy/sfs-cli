package db

import (
	"database/sql"
	"fmt"

	"gorm.io/driver/mysql"
	"gorm.io/driver/postgres"
	"gorm.io/driver/sqlite"
	"gorm.io/gorm"

	"github.com/fossMeDaddy/sfs-cli/constants"
	"github.com/fossMeDaddy/sfs-cli/sfserver/db/models"
	_ "github.com/go-sql-driver/mysql"
	_ "github.com/jackc/pgx/v5/stdlib"
	_ "github.com/mattn/go-sqlite3"
)

func ConnectGormDb(driver, connUri string, autoMigrate bool) (*gorm.DB, error) {
	var localDb *gorm.DB

	switch driver {
	case constants.DbDriverPg:
		db, err := sql.Open("pgx", connUri)
		if err != nil {
			return localDb, err
		}

		gDb, err := gorm.Open(postgres.New(postgres.Config{
			Conn: db,
		}))
		if err != nil {
			return gDb, err
		}

		localDb = gDb

	case constants.DbDriverMySql:
		db, err := sql.Open("mysql", connUri)
		if err != nil {
			return localDb, err
		}

		gDb, err := gorm.Open(mysql.New(mysql.Config{
			Conn: db,
		}))
		if err != nil {
			return gDb, err
		}

		localDb = gDb

	case constants.DbDriverSqlite:
		db, err := sql.Open("sqlite3", connUri)
		if err != nil {
			return localDb, err
		}

		gDb, err := gorm.Open(sqlite.New(sqlite.Config{
			Conn: db,
		}))
		if err != nil {
			return gDb, err
		}

		localDb = gDb

	default:
		return nil, fmt.Errorf("unsupported database driver '%s' entered!\n", driver)
	}

	if localDb == nil {
		return nil, fmt.Errorf("gorm.DB is a nil ptr")
	}

	if autoMigrate {
		localDb.AutoMigrate(
			&models.User{},
			&models.ApiKey{},
			&models.ApiKeyUsageSummary{},
			&models.FsFile{},
			&models.FileSystem{},
		)
	}

	return localDb, nil
}
