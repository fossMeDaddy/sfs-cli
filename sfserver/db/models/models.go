package models

import (
	"time"

	"gorm.io/gorm"
)

type User struct {
	GhUsername string `gorm:"primaryKey"`
	GhToken    string `gorm:"unique"`
	Email      string `gorm:"unique"`

	CreatedAt time.Time
}

type ApiKey struct {
	Key    string `gorm:"primaryKey"`
	Secret string `gorm:"unqiue"`
	UserId string `gorm:"unique"`

	FreeReads                int64
	FreeWrites               int64
	FreeStorageGb            int64
	FreeQuotaIntervalSeconds int64

	CreatedAt time.Time
	UpdatedAt time.Time

	User User `gorm:"foreignKey:UserId;references:GhUsername;constraint:OnDelete:CASCADE"`
}

type ApiKeyUsageSummary struct {
	gorm.Model

	ApiKeyId  string    `gorm:"uniqueIndex:idx_key_startdate_enddate"`
	StartDate time.Time `gorm:"uniqueIndex:idx_key_startdate_enddate"`
	EndDate   time.Time `gorm:"uniqueIndex:idx_key_startdate_enddate"`

	ReadCalls  int64
	WriteCalls int64

	ApiKey ApiKey `gorm:"foreignKey:ApiKeyId;references:Key;constraint:OnDelete:CASCADE;"`
}

type FileSystem struct {
	Id              string `gorm:"primaryKey"`
	StorageRootPath string `gorm:"unique"`

	ApiKey      ApiKey `gorm:"foreignKey:Id;references:Key;constraint:OnDelete:CASCADE;"`
	DirTreeJson string
}

type FsFile struct {
	// directly corresponds to the id in actual storage, can be access with FileSystem.StorageRootPath + / + StorageId
	StorageId string `gorm:"primaryKey"`

	FileSystemId string `gorm:"index"`
	DirId        string `gorm:"uniqueIndex"` // dirtree dir id, will be a uuid so doesn't really matter anyway
	Name         string // full filename (e.g. "bikini.json", "my_binary") OR same as storage id, anyways, only for display

	FileSize    *int64  // null if path_type represents a dir
	FileType    *string // null if path_type represents a dir, same as the content-type header value at upload time (only for named distribution)
	IsEncrypted bool
	IsPublic    bool

	// hand-off actual deletion to a CRON job for making sure file gets delete from storage as well
	DeletedAt *time.Time
}
