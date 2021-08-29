package main

import (
	"fmt"
	"log"
	"net/http"
	_ "net/http/pprof"
	"os"
	"sync"
	"unsafe"
)

// #cgo amd64 386 CFLAGS: -DX86=1
// #cgo LDFLAGS: -L${SRCDIR} -leafcc
// #include <stdlib.h>
// #include <eafcc.h>
import "C"

type CFGCenter struct {
	cc unsafe.Pointer
}

type CFGContext struct {
	ctx unsafe.Pointer
}

func NewCfgCenter(cfg string) *CFGCenter {
	ccfg := C.CString(cfg)
	defer C.free(unsafe.Pointer(ccfg))

	ret := CFGCenter{}

	if handler := C.new_config_center_client(ccfg); handler != nil {
		ret.cc = unsafe.Pointer(handler)
		return &ret
	}
	return nil
}

func (c *CFGCenter) GetCfg(ctx *CFGContext, key string) (string, string) {
	ckey := C.CString(key)
	defer C.free(unsafe.Pointer(ckey))

	t := C.get_config((*C.eafcc_CFGCenter)(c.cc), (*C.eafcc_Context)(ctx.ctx), ckey)
	contextType := C.GoString(t.content_type)
	value := C.GoString(t.value)
	C.free_config_value((*C.eafcc_ConfigValue)(t))
	return contextType, value
}

func NewContext(ctx string) *CFGContext {
	cctx := C.CString(ctx)
	defer C.free(unsafe.Pointer(cctx))

	ret := CFGContext{}

	if handler := C.new_context(cctx); handler != nil {
		ret.ctx = unsafe.Pointer(handler)
		return &ret
	}
	return nil
}

func main() {
	go func() {
		log.Println(http.ListenAndServe("localhost:6060", nil))
	}()

	dir, _ := os.Getwd()
	fmt.Println(dir)
	cc := NewCfgCenter(`{
		"storage_backend": {
			"type": "filesystem",
			"path": "../../test/mock_data/filesystem_backend/"
		}
	}`)

	ctx := NewContext("foo=123\nbar=456")

	wg := sync.WaitGroup{}
	wg.Add(4)
	for i := 0; i < 4; i++ {
		go func() {
			defer wg.Done()
			for x := 0; x < 1000000; x++ {
				contextType, value := cc.GetCfg(ctx, "my_key")
				if contextType != "application/json" {
					panic(contextType)
				}
				if value != `{"aaa":[{},{"bbb":"hahaha"}]}` {
					panic(contextType)
				}
			}
		}()
	}

	wg.Wait()

}
