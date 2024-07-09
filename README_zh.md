# imx-rs
基于 rust 语言实现的 MTProto 即时通信客户端, 目前在极慢速开发中, 看心情

### crates
- **imx**: 对外提供依赖, re-export crates
- **[imx_core](crates/imx_core/README.md)**: 核心逻辑, 实现基础网络通信, 包括网络, 协议, 序列化/反序列化
- **imx_file**: 普通文件扩展, 包括发送接收文件
- **imx_geo**: 地理位置扩展
- **imx_media**: 多媒体扩展, 包括图片, 音视频编解码, 发送接收
- **imx_webrtc**: WebRTC 实时音视频通话
- **serde_mt**: 实现基于 [serde](https://crates.io/crates/serde) 框架的 MTProto 序列化/反序列化实现
- **with_crc**: 在序列化和反序列化中注入 crc
- **with_crc_derive**: 自动实现 with_crc trait 的注解处理器
