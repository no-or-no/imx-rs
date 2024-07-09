# imx_core
实现基础网络通信, 包括网络, 协议, 序列化/反序列化

### MTProto
[Mobile Transport Protocol](https://core.telegram.org/mtproto/description#schematic-presentation-of-messages)

#### 术语:
- **Authorization Key (auth_key)**
  客户端和服务器共享的 2048-bit (256 bytes)密钥，通过 Diffie-Hellman 密钥交换获得，永远不会通过网络传输。
  每个 `auth_key` 都是用户专有的，用户可以有多个 `auth_key` (对应到不同设备上的持久化 Session)，
  如果设备丢失，对应的 `auth_key` 可能会被永久锁定。

- **Server Key**
  2048-bit (256 bytes)的 RSA 密钥，在注册中和生成 `auth_key` 时，服务器用来对自己的消息进行数字签名。
  应用程序有一个内置的公钥，可用于验证签名，但不能用于消息签名。私钥存储在服务器上，并且很少更改。

- **Key Identifier (auth_key_id)**
  `auth_key` 的 SHA1 哈希值的低 64 bits (8 bytes)，用于指定哪个 `auth_key` 被用于消息加密。
  `auth_key_id` 与 `auth_key` 要一一对应，如果 `auth_key_id` 出现碰撞, 需要重新生成 `auth_key`。
  `auth_key_id` 为 0 表示不加密，只有在生成 `auth_key` 的 Diffie-Hellman 密钥交换过程中的消息类型中用到。
  `auth_key_id` 与 MTProto 协议版本无关。

- **Session**
  客户端生成的一个 64-bit (8 bytes)随机数，用于区分各个 Session (例如，在使用相同 `auth_key` 创建的应用程序的不同实例之间)。
  Session 与 `auth_key_id` 一起对应于一个应用程序实例。服务器可以维护 Session 状态。在任何情况下，
  一个 Session 的消息不能发送到另一个 Session 中。服务器可能单方面忘记任何客户端 Session，客户端需要处理这种情况。

- **Server Salt**
  一个 64-bit (8 bytes) 随机数(独立于 Session)，服务器要求每 30 分钟更改一次。所有后续消息都必须包含新 salt (服务器在之后的
  1800 秒内仍然接受带有旧 salt 的消息)。用于防止重播攻击和调整客户端时间到未来某个时刻的把戏。

- **消息 id (msg_id)**
  (与时间相关的) 64-bit (8 bytes)数，用于在一个 Session 中唯一标识一条消息。客户端发送的 `msg_id` 可被 4 整除，服务器对客户端的
  响应 `msg_id` 模 4 余 1，服务端其它 `msg_id` 余 3。客户端 `msg_id` 与服务器 `msg_id` 一样，(在单个 Session 中)
  必须单调递增，并且必须约等于 unixtime * (1 << 32)。这样，`msg_id` 能表示消息创建的大致时间。
  消息在创建后 300 秒或创建前 30 秒被拒绝 (防止重放攻击)。在这种情况下，必须使用不同的 `msg_id` 重新发送消息
  (或者将消息放在具有更高 `msg_id` 的容器类型消息中)。容器类型消息的 `msg_id` 必须严格大于其内部消息的 `msg_id`。
  > **重要提示** 为了对抗重放攻击，客户端传递的 `msg_id` 的低 32 bits 不能为空，必须为创建消息时时间点的小数部分。

- **与内容相关的消息 (ack)**
  需要明确确认的消息。这些消息包括所有用户和许多服务类消息，除了容器和 `ack` 消息之外的几乎所有消息。

- **消息序列号 (msg_seqno)**
  32-bit (4 bytes)数，等于发送方在此消息之前创建的 `ack` 消息数的 2 倍，如果当前消息是 `ack` 消息，则 +1。容器消息总是在其全部内容之后生成，
  因此，其 `msg_seqno` >= 其中包含的消息的 `msg_seqno`。

- **Message Key (msg_key)**
  待加密消息(包括 internal header 和 padding bytes)的 SHA-256 哈希值的中间 128 bits (16 bytes), 前面加 `auth_key` 的 32 bytes 片段。

- **Internal header**
  `Server salt` (64 bits) + `Session` (64 bits), 放在消息前面与消息一起加密

- **External header**
  `auth_key_id` (64 bits) + `msg_key` (128 bits), 放在加密后的消息前面

- **Payload**
  `External header` + 加密后的消息

#### Aes key 和 iv
`aes_key` (256-bit) 和 `aes_iv` (256-bit) 用于消息 AES-256 IGE 模式加密.

**计算步骤:**
> x = 0 客户端发送到服务端的消息  
> x = 8 服务端发送到客户端的消息
- msg_key_large = SHA256(substr(auth_key, 88+x, 32) + plaintext + random_padding)
- msg_key = substr(msg_key_large, 8, 16)
- sha256_a = SHA256(msg_key + substr(auth_key, x, 36))
- sha256_b = SHA256(substr(auth_key, 40+x, 36) + msg_key)
- aes_key = substr(sha256_a, 0, 8) + substr(sha256_b, 8, 16) + substr(sha256_a, 24, 8)
- aes_iv = substr(sha256_b, 0, 8) + substr(sha256_a, 8, 16) + substr(sha256_b, 24, 8)

#### 握手
- C -> S: ReqPQMulti 
- S -> C: ResPQ 
- C -> S: ReqDHParams 
- S -> C: ServerDHParams 
- C -> S: SetClientDHParams 
- S -> C: SetClientDHParamsAnswer