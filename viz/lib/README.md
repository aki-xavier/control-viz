# papers/viz/lib — 内嵌的 three.js(r160)

播放器页面 `index.html` / `poke.html` 直接以 ES module 引用这里的三份文件,
页面因此**离线可用**:没有构建步骤,也没有运行时 CDN。

上游是 npm 包 `three@0.160.0` 的官方分发,逐字节一致(sha256 已核对):

| 文件 | 上游路径 | sha256 |
|---|---|---|
| `three.module.js` | `build/three.module.js` | `76dea8151bc9352aef3528b4262e249b2604f62543828328db978d060d61a495` |
| `OrbitControls.js` | `examples/jsm/controls/OrbitControls.js` | `5a44a9e86a2a0fb11933eed69bc2cd33c76a496854c1aed6ed776efa87d7b064` |
| `STLLoader.js` | `examples/jsm/loaders/STLLoader.js` | `896d006a48b8f125385a485ccae154dadee801a953f0b45ceffe7ddd8a29ca93` |

`three.module.js` 自带许可证头(MIT,Copyright 2010-2023 Three.js Authors;
`OrbitControls.js` / `STLLoader.js` 是同一分发的 jsm 模块,不带独立文件头)。

升级时替换这三份文件并更新上表的 sha256;`REVISION` 常量在 `three.module.js`
开头(`'160'`),可用来确认版本。
