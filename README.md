<div align="center">

<samp>

# zip-pw-finder

**A password finder for protected ZIP files using a brute force strategy**

</samp>

</div>

## Architecture

```mermaid
graph LR;

A[Password generator thread]
B([Channel])
1[Worker thread]
2[Worker thread]
3[Worker thread]
4[Worker thread]

A -->|push| B
B --> 1
B --> 2
B --> 3
B --> 4
```

## Dependencies

- [crossbeam-channel](https://github.com/crossbeam-rs/crossbeam)
- [zip-rs](https://github.com/zip-rs/zip)
- [indicatif](https://github.com/console-rs/indicatif)
