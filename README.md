pty-for-each
------------

`pty-for-each` is a line-oriented TTY multiplexer, that allows spawning several
processes, collecting their emitted terminal lines and print them interleaved
with a prefix. One variable can be provided as a form of a list of known values
in order to provide variation in execution.

It also allows running a single process under a terminal just to record its
output in a situation where a terminal is absent.

For example, invoking ssh to run `wget` on two hosts in parallel

```
$ pfe key host -- mars centos-7 %% ssh  %host 'wget www.cnn.com -O /dev/null'
centos-7: --2018-01-26 08:35:30--  http://www.cnn.com/
centos-7: Resolving www.cnn.com (www.cnn.com)... 151.101.65.67, 151.101.129.67, 151.101.193.67, ...
centos-7: Connecting to www.cnn.com (www.cnn.com)|151.101.65.67|:80... connected.
mars: --2018-01-26 17:52:45--  http://www.cnn.com/
mars: Resolving www.cnn.com (www.cnn.com)... 151.101.193.67, 151.101.65.67, 151.101.1.67, ...
centos-7: HTTP request sent, awaiting response... 301 Moved Permanently
centos-7: Location: https://www.cnn.com/ [following]
centos-7: --2018-01-26 08:35:30--  https://www.cnn.com/
mars: Connecting to www.cnn.com (www.cnn.com)|151.101.193.67|:80... connected.
mars: HTTP request sent, awaiting response... 301 Moved Permanently
mars: Location: https://www.cnn.com/ [following]
mars: --2018-01-26 17:52:45--  https://www.cnn.com/
centos-7: Connecting to www.cnn.com (www.cnn.com)|151.101.65.67|:443... connected.
mars: Connecting to www.cnn.com (www.cnn.com)|151.101.193.67|:443... connected.
centos-7: HTTP request sent, awaiting response... 302 Found
centos-7: Location: https://edition.cnn.com/ [following]
centos-7: --2018-01-26 08:35:31--  https://edition.cnn.com/
centos-7: Resolving edition.cnn.com (edition.cnn.com)... 151.101.65.67, 151.101.193.67, 151.101.129.67, ...
centos-7: Connecting to edition.cnn.com (edition.cnn.com)|151.101.65.67|:443... connected.
mars: HTTP request sent, awaiting response... 302 Found
mars: Location: https://edition.cnn.com/ [following]
mars: --2018-01-26 17:52:46--  https://edition.cnn.com/
mars: Resolving edition.cnn.com (edition.cnn.com)... 151.101.129.67, 151.101.1.67, 151.101.193.67, ...
mars: Connecting to edition.cnn.com (edition.cnn.com)|151.101.129.67|:443... connected.
centos-7: HTTP request sent, awaiting response... 200 OK
centos-7: Length: 159217 (155K) [text/html]
centos-7: Saving to: ‘/dev/null’
centos-7:
centos-7:      0K .......... .......... .......... .......... .......... 32%  308K 0s
mars: HTTP request sent, awaiting response... 200 OK
mars: Length: 35351 (35K) [text/html]
mars: Saving to: ‘/dev/null’
mars:
centos-7:     50K .......... .......... .......... .......... .......... 64%  899K 0s
centos-7:    100K .......... .......... .......... .......... .......... 96% 2.05M 0s
centos-7:    150K .....                                                 100%  141M=0.2s
centos-7:
centos-7: 2018-01-26 08:35:31 (643 KB/s) - ‘/dev/null’ saved [159217/159217]
centos-7:
mars:      0K .......... .......... .......... ....                 100%  333K=0.1s
mars:
mars: 2018-01-26 17:52:46 (333 KB/s) - ‘/dev/null’ saved [159217]
mars:
```

### Limitation

* User input from the terminal is ignored. Theoretically, it can be distributed to
all sub-terminals.

### Installation

Currently `pty-for-each` can be built from sources using `cargo` from the installable
[Rust][https://www.rust-lang.org/en-US/install.html] compiler toolchain. After installing
the toolchain, simply run `cargo install` in the repository directory.
