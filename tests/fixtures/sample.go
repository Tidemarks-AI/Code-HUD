package main

import (
	"fmt"
	"strings"
)

const MaxRetries = 3

var DefaultTimeout = 30

type Config struct {
	Host string
	Port int
}

type Handler interface {
	Handle(req string) string
	Close() error
}

type status int

func NewConfig(host string, port int) *Config {
	return &Config{Host: host, Port: port}
}

func (c *Config) Address() string {
	return fmt.Sprintf("%s:%d", c.Host, c.Port)
}

func helper(s string) string {
	return strings.TrimSpace(s)
}

func TestExample() {
	fmt.Println("test")
}
