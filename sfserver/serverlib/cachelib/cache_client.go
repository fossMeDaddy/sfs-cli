package cachelib

import (
	"sync"

	"github.com/valkey-io/valkey-go"
)

type MemCache = map[string]interface{}

type CacheClient struct {
	valkeyClient valkey.Client
	memCache     *MemCache
	mx           sync.Mutex
}

// if connUri is empty, a cache datastructure is initialized in-memory
func NewCacheClient(connUri string) (*CacheClient, error) {
	cacheClient := &CacheClient{}

	if len(connUri) > 0 {
		opt, optErr := valkey.ParseURL(connUri)
		if optErr != nil {
			return cacheClient, optErr
		}

		client, clientErr := valkey.NewClient(opt)
		if clientErr != nil {
			return cacheClient, clientErr
		}

		cacheClient.valkeyClient = client
		return cacheClient, nil
	}

	cacheClient.memCache = &MemCache{}
	return cacheClient, nil
}
