# Reflow: One proxy to rule them all

This program let you make full use of all your proxies, VPNs, and interfaces, automatically.

## Features
- Route ip packets to proxies

    It operates on the network layer, so you can stop worrying about proxy support in applications.
    You can use a socks5 proxy wherever you want. 
    And you won't forget to use your privacy-enhanced VPNs, ever.

- Detect protocol metadata for finer-grained control

  In addtion to ip address and port, protocol (HTTP, TLS, SSH, etc.) metadata (domain name, user-agent, etc.) are detected.

- Prefix-match domain names and ip subnets

  Use a trie or prefix tree to sort domains and addresses into zones, because they have a natural hierarchical ownership structure

- Use a tree diagram to configure any routing strategy

  Any decision-making process that can be expressed as a cascade of conditional statements can be used. Use all the protocol information to make intelligent routing decisions. Privacy, speed, low cost, choose any three.

- Selectively proxy DNS queries depending on the domain name

- Drop traffic to domains or ip address known to serve only ads and tracking

  It handles tens of thousands of domains with ease even on an OpenWrt device

# Installation

After you have the nightly version of [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) installed.

Clone the project `git clone https://github.com/net-reflow/reflow`

and run `cargo install`, the binary will be installed in `~/.cargo/bin/reflow`

You can now run it using `reflow --config path`, where `path` is the directory containing all the configuration

A good starting point for the configuration is [reflow.conf](https://github.com/net-reflow/reflow.conf)

# What does the Decision Tree do in a proxy?

This is where the power of `reflow` shows, you'll get a basic idea by looking at an example with comments:

    any[
        # when your computer wants to make a connection to the internet
        # reflow inspects the first packets, thoroughly
        # first, it checks the domain, (if the application layer protocol uses a domain)
        cond domain {
          # if it's listed as one of "secret-sites" (including sub-domains) in configuration
          # use the proxy defined as privacyproxy
          secret-sites => privacyproxy
          # block traffic to known ad servers by domain
          # "reset" is a built-in option, which means drop the connection
          adservers => reset
          # you can chain rules, the following will only match when the domain is in https-only and the protocol is http
          # otherwise, this "cond domain" section doesn't match, and rules following it will be tried
          https-only => cond protocol {
            http => reset
          }
        }

        # next look at ip addresses
        cond ip {
            # you can use workvpn0 to access your working environment
            worknet => workvpn0
            # "direct" is a another built-in option, meaning use the existing default route 
            homelan => direct
        }
        # if the rules above hasn't produced a match, continue to check the protocol
        cond protocol {
          ssh => any [
            cond ip  {
            # some ssh hosts may be only accessible through a certain proxy
                mars => moon
            }
            # another example of combing conditions: when the protocol is ssh AND the port is 22
            cond port eq 22 => proxy1
            # this will always match sucessfully for ssh traffic
            direct
          ]
        }
        # catch-all rule for everything else
        direct
    ]

The enclosing `any[` and `]` means rules listed inside it will be tried one by one

# Configuration

Example configuration and documentation is provided at
 [reflow.conf](https://github.com/net-reflow/reflow.conf)

# Contributing

Please try it and give any kind of feedback by opening issues

## Development

Here're some features being developed or considered:

* Built-in tun support, add UDP support
* Support more protocols
* Chaining proxies
* Add Dns over https or tls support
* Add Dns cache

## Make a Donation

BTC: `bc1q8cxs2e3wf525f958zgxzq4skl94nfzwuuq97qz`
