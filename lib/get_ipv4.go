package lib

import (
	"net"
)

func GetIpv4() (string, error) {
	conn, err := net.Dial("udp4", "1.1.1.1:80")
	if err != nil {
		return "", err
	}
	conn.Close()

	ipv4 := conn.LocalAddr().(*net.UDPAddr)

	return ipv4.IP.To4().String(), nil
}
