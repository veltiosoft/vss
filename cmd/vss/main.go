package main

import (
	"log"
	"os"

	"github.com/mitchellh/cli"
	"github.com/vssio/go-vss"
)

func buildCommandFactory() (cli.Command, error) {
	return &vss.BuildCommand{}, nil
}

func serveCommandFactory() (cli.Command, error) {
	return &vss.ServeCommand{}, nil
}

func main() {
	c := cli.NewCLI("vss", "0.0.1")
	c.Args = os.Args[1:]
	c.Commands = map[string]cli.CommandFactory{
		"build": buildCommandFactory,
		"serve": serveCommandFactory,
	}

	exitStatus, err := c.Run()
	if err != nil {
		log.Println(err)
	}

	os.Exit(exitStatus)
}
