# Reflow: One proxy to rule them all

This program let you make full use of all your proxies, VPNs, and interfaces, automatically.

## Features
- Route ip packets to proxies

    It operates on the network layer, so you can stop worrying about proxy support in applications. And you won't forget to use your privacy-enhanced VPNs, ever.

- Detect protocol metadata for finer-grained control

  In addtion to ip address and port, protocol (HTTP, TLS, SSH, etc.) metadata (domain name, user-agent, etc.) are detected.

- Prefix-match domain names and ip subnets

  Use a trie or prefix tree to sort domains and addresses into zones, because they have a natural hierachical ownership structure

- Use a tree diagram to configure any decision-making process

  Any decision-making process that can be expressed as a cascade of conditional statements can be used. Use all the protocol information to make intelligent routing decisions. Privacy, speed, low cost, choose any three.

# Installation

After you have the nightly version of [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) installed.

Clone the project `git clone https://github.com/net-reflow/reflow`

and run `cargo install`, the binary will be installed in `~/.cargo/bin/reflow`

You can now run it using `reflow --config path`, where path is the directory containing all the configuration

# Configuration

## Decision Tree

This is the most unique part, you'll get a rough idea just by looking at an example:


    Tree-Format: reflow 0.1
    
    cond domain {
      secret-sites => use privacyproxy
      https-only => cond protocol {
        http => do reset
      }
      adservers => do reset
    }
    
    cond ip {
        worknet => use workvpn0
        homelan => do direct
    }
    cond protocol {
      ssh => any [
        cond ip => {
            mars => use moon
        }
        cond port eq 22 => use proxy1
        do direct
      ]
    }
    do direct

The decision tree configuration is saved in the file `tcp.reflow`.

### The idea

At the top level, rules are tried from top to bottom, until a match is found.

A rule contains zero or more conditions and one final decision. They can be thought of as nodes and leaf nodes in a tree.

When conditions are chained, they express the "logical AND" relation.

Rules can be combined sequentially, too. In this case they'll be tried one by one.

### Syntax

The first line records the version of the format for forward compatibility.

The configuration is formatted with parentheses and newlines, indentations are optional.

#### Conditions

Conditions start with the keyword `cond`, followed by the type, which is one of `domain`, `ip`, `protocol`, or `port`.

The first three are mappings, which are like hash maps, with keys and values separated by `=>`.
The "keys" here are "zones" in case of `domain` and `ip`, they are arbitrary names given to groups of domains and ips. In case of `protocol`, it can be just one of the recognized protocols: `ssh`, `http`, `tls`.

The `port` condition just tests whether it's equal to a particular number.

#### Right-hand side

In both cases, the right-hand side is another rule, which can be another condition, making a logical "AND"; or a final decision, making a match, finishing the decision-making process; or a sequence or more rules to try one by one.

When you want to combine rules sequentially, put them in an `any` block, enclosed in square brackets(`[ ]`).

#### Final decision

It usually starts with the keyword `use`, followed by the name of a configured route.
Simple actions are written inline, starting with the keyword `do`, including `do direct` and `do reset`.

### Summary
The configuration above can be translated into natural language as:
  - If the domain name is one of "secret-sites", use proxy named "privacyproxy"; if it's "https-only" and the protocol is http, reject; if it's in "adservers", reject.
  - Use "workvpn0" to connect to ips belonging to "worknet", connect directly to ip in "homelan"
  - Make ssh connections to ip in "mars" with proxy "moon"; otherwise use "proxy1" when the port is 22 and use the default route otherwise
  - Use the default route for everything else

## Zones

Domain names and ip addresses are first sorted into named zones by listing them in plaintext files.

To configure an ip zone named "worknet", create a directory named "ipregion.worknet", inside it create any number of text files with any file name. Write one subnet per line in cidr notation,
such as `192.168.100.0/22`.
Write the address length when it's just a single address,
such as `208.130.29.33/32`.

To configure a domain name zone named "https-only", write text files in the directory "region.https-only". Domain names should start from the root, such as "com.google.www", this is in contrast to most web browsers.

## Other configurations

These are located in the file `config.toml` in the configuration directory.

### Relay

This is the main service. There're two options, both accepting a socket address.

`listen_socks` configures the port on which to run a socks server
`resolver` is the dns server to use when socks clients request to connect to a domain instead of an ip.

Example:

    [relay]
    resolver="127.0.0.1:5353"
    listen_socks="127.0.0.1:1080"

### tun2socks

Built-in tun support is still work-in-progress, in the meantime, please get [tun2socks](https://github.com/ambrop72/badvpn/wiki/Tun2socks) and point it to the socks server configured above with option `--socks-server-addr 127.0.0.1:1080`.

### Gateways

These are proxies and interfaces configuration. Supported kinds are socks, which require the host and port of a socks server, and bind, which is the address of the network interface you want to use.

This is how this program make use of VPNs. You need to start a VPN, but don't let it override the default route. Configure routes and rules in such a way that a VPN is only used when a program binds to the interface before making a connection.
```
[gateway.privacyproxy]
kind = "socks5"
host = "127.0.0.1"
port = 2123
[gateway.workvpn0]
kind="bind"
ip="192.168.33.83"
```

### Built-in DNS Proxy

The DNS proxy forwards DNS requests to different upstream servers (possibly through a proxy), depending on what zone the queried domain is in.


    [dns]
    listen="127.0.0.1:5353"
    [dns.server.quadone]
    addr="1.1.1.1:53"
    [dns.server.localisp]
    addr="192.168.1.1:53"
    [dns.rule]
    secret-sites="quadone"
    else="localisp"


The rule here is one simple mapping from the zone to the dns server to use. There must be a special key `else`, which is the default dns to use.

# Contributing

Please try it and give any kind of feedback by opening issues

## Development

Here're some features being developed or considered:

* Built-in tun support, add UDP support
* Support more protocols
* Simplify configuration files: replace toml, which isn't very concise in some cases
* Chaining proxies
* Add Dns over https or tls support
* Add Dns cache

## Make a Donation

Coming soon.
