package main

import (
	"log"
	"os"

	"github.com/vssio/go-vss"
)

func main() {
	c := vss.NewCLI()

	exitStatus, err := c.Run()
	if err != nil {
		log.Println(err)
	}

	os.Exit(exitStatus)
}
