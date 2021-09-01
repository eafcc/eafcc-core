package main

import (
	"fmt"
	"log"
	"net/http"
	_ "net/http/pprof"
	"os"
	"runtime"
	"sync"
	"unsafe"
)

// #cgo amd64 386 CFLAGS: -DX86=1
// #cgo LDFLAGS: -L${SRCDIR} -leafcc
// #include <stdlib.h>
// #include <eafcc.h>
// void update_cb_c(void *user_data);
// void update_cb_go(void *user_data);
// typedef void (*eafcc_update_cb_fn)(void*);
import "C"
type CFGCenter struct {
	cc unsafe.Pointer
}

type CFGContext struct {
	ctx unsafe.Pointer
}

//export update_cb_go
func update_cb_go(userData unsafe.Pointer) {
	print("cb in go")
}

func NewCfgCenter(cfg string) *CFGCenter {
	ccfg := C.CString(cfg)
	defer C.free(unsafe.Pointer(ccfg))

	ret := CFGCenter{}

	pp := uintptr(1000)
	if handler := C.new_config_center_client(
		ccfg,
		(C.eafcc_update_cb_fn)(unsafe.Pointer(C.update_cb_go)),
		unsafe.Pointer(pp),
		); handler != nil {
		ret.cc = unsafe.Pointer(handler)

		return &ret
	}
	return nil
}

func (c *CFGCenter) GetCfg(ctx *CFGContext, keys []string) [][2]string {
	if len(keys) == 0 {
		return nil
	}

	ckeys := make([]unsafe.Pointer, 0, len(keys))
	for _, key := range keys {
		ckey := C.CString(key)
		ckeys = append(ckeys, unsafe.Pointer(ckey))
		defer C.free(unsafe.Pointer(ckey))
	}
	
	t := C.get_config((*C.eafcc_CFGCenter)(c.cc), (*C.eafcc_Context)(ctx.ctx), (**C.char)(unsafe.Pointer(&ckeys[0])), C.ulong(len(keys)))
	

	ret := make([][2]string, 0, len(keys))

	for i:=0; i<len(keys); i++ {
		tmpP := unsafe.Pointer(uintptr(unsafe.Pointer(t)) + uintptr(i) * unsafe.Sizeof(C.eafcc_ConfigValue{}))
		t := (*C.eafcc_ConfigValue)(tmpP)
		contextType := C.GoString(t.content_type)
		value := C.GoString(t.value)
		ret = append(ret, [2]string{contextType, value})
	}
	C.free_config_value(t, C.ulong(len(keys)))
	return ret
}

func NewContext(ctx string) *CFGContext {
	cctx := C.CString(ctx)
	defer C.free(unsafe.Pointer(cctx))

	ret := &CFGContext{}

	if handler := C.new_context(cctx); handler != nil {
		ret.ctx = unsafe.Pointer(handler)

		runtime.SetFinalizer(ret, func(w *CFGContext) {
			C.free_context((*C.eafcc_Context)(w.ctx))
		})

		return ret
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
	
	
	wg := sync.WaitGroup{}
	wg.Add(1)
	for i := 0; i < 1; i++ {
		go func() {
			defer wg.Done()
			ctx := NewContext("foo=123\nbar=456")
			for x := 0; x < 1; x++ {
				if x % 10000 == 0 {
					runtime.GC()
				}
				
				values := cc.GetCfg(ctx, []string{"my_key", "my_key", "my_key", "my_key"})
				
				contextType, value := values[0][0], values[0][1]
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

	// time.Sleep(1000*time.Hour)

}
