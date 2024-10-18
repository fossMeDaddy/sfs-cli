package serverglobals

import (
	"github.com/fossMeDaddy/sfs-cli/sfserver/serverlib/cachelib"
	"gorm.io/gorm"
)

var (
	DB          *gorm.DB
	CacheClient *cachelib.CacheClient
)
