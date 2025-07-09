# wolly

wolly is a transparent reverse TCP proxy that uses Wake-On-Lan to wake up the
target machine(s). It accepts incoming connections and forwards them to the
configured host after waking up the host with Wake-On-Lan.

wolly is configured with only one file. This file contains rules as to where and
how wolly should forward connections. It uses a line-based format that contains
"directives". An example configuration:

```
upstream 10.0.0.42 mac 12:34:56:78:9a:bc brd 10.0.0.255
upstream 192.168.1.42 mac 00:00:00:00:00:00 brd 192.168.1.255

forward 0.0.0.0:8080 to 10.0.0.42:28080
forward 127.0.0.1:9999 to 10.0.0.42:9999
forward 0.0.0.0:10000-10100 to 192.168.1.42:2200-2300
```

The above configuration declares that there are 2 "upstream" hosts
`10.0.0.42` and `192.168.1.42` which are available to forward connections
to. It also declares that wolly should forward connections from port 8080 to
`10.0.0.42:28080`, `9999` to `10.0.0.42:9999` and every port in the range
`10000-10100` to `192.168.1.42:2200-2300`. wolly supports 3 types of forwarding:

  1. 1-1, one port is forwarded to another
  2. N-1, many ports are forwarded to one
  3. N-N, a port range is forwarded to another port range

What makes wolly different than other TCP proxies is that wolly will attempt to
wake up the target host before it forwards the connection.

A full configuration file is available here: [./wolly.conf](./wolly.conf)
