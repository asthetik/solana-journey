### 什么是区块链？

> A blockchain is a distributed ledger with growing lists of records (blocks) that are securely linked together via cryptographic hashes. Each block contains a cryptographic hash of the previous block, a timestamp, and transaction data (generally represented as a Merkle tree, where data nodes are represented by leaves). Since each block contains information about the previous block, they effectively form a chain (compare linked list data structure), with each additional block linking to the ones before it. Consequently, blockchain transactions are resistant to alteration because, once recorded, the data in any given block cannot be changed retroactively without altering all subsequent blocks and obtaining network consensus to accept these changes.
>
> Blockchains are typically managed by a peer-to-peer (P2P) computer network for use as a public distributed ledger, where nodes collectively adhere to a consensus algorithm protocol to add and validate new transaction blocks. Although blockchain records are not unalterable, since blockchain forks are possible, blockchains may be considered secure by design and exemplify a distributed computing system with high Byzantine fault tolerance.

区块链是一种去中心化的分布式账本，通过加密算法、共识机制和分布式存储等技术，确保数据不可篡改、公开透明且可追溯。

####  核心特性

- **去中心化**
  数据不依赖于单一机构存储或验证，而是由网络中的多个节点共同维护，避免了中心化控制的风险。
- **不可篡改**
  数据按时间顺序以“区块”形式链接，每个区块包含前一个区块的哈希值，修改任一区块会导致后续所有区块失效，需获得全网多数节点共识，几乎无法篡改。
- **透明可追溯**
  所有交易记录公开可见，任何操作均可追溯来源。
- **共识机制**
  节点通过算法（如工作量证明PoW、权益证明PoS）达成对数据一致性的认可，确保无需信任中介即可协作。

#### 基本运作原理

1. **交易发起**
   用户发起交易（如转账），交易数据经加密后广播至网络。
2. **区块打包**
   节点（矿工或验证者）将交易打包成区块，并验证其有效性。
3. **共识确认**
   网络通过共识机制竞争记账权，获胜者将新区块添加到链上，并同步给所有节点。
4. **链式存储**
   新区块通过哈希值与前一个区块链接，形成连续链条。



### 分布式系统

分布式系统是由多台独立的计算机节点通过网络连接，协同工作，看起来像一个单一连贯系统的计算模型，它通过并行处理、资源共享和去中心化设计，实现高可用性、可伸缩性和高性能，广泛应用于云计算、大型互联网应用等，解决单机系统瓶颈和故障问题，核心在于透明性、并发性、无全局时钟和独立故障处理。 

核心概念

- **定义**: 多个节点（计算机、进程）组成，通过消息传递通信，共同完成一个目标。
- **透明性**: 用户感知不到系统是由多台机器组成，对用户而言是单一系统。
- **分布性**: 组件物理上分散，提高容错性。
- **并发性**: 节点并行工作，提高吞吐量。
- **无全局时钟**: 各节点有本地时钟，存在偏差，带来挑战。
- **独立故障**: 节点故障不影响整个系统，实现高可用。 

解决的问题

- **解决单点故障**: 传统系统一台机器故障整个系统就停，分布式系统有冗余，更可靠。

- **突破单机性能瓶颈**: 任务分解到多台机器并行计算，处理海量数据和高并发。

- **实现水平扩展**: 通过增加更多节点（而不是升级单机硬件）来提升能力。 

### CAP 定理

在理论计算机科学中，**CAP 定理**（CAP theorem），又被称作**布鲁尔定理**（Brewer's theorem），它指出对于一个分布式计算系统来说，不可能同时满足以下三点：

- 一致性（Consistency）：所有节点访问同一份最新的数据副本

- 可用性（Availability）：每次请求都能获取到非错的响应——但是不保证获取的数据为最新数据

- 分区容错性（Partition tolerance）：系统能够在网络故障时将服务器分隔成孤立组继续运行。

我们必须保证分区容错性，因为网络分区是不可避免的：电缆会被切断，路由器会失效，数据中心会断电。这使我们需要在一致性和可用性之间做出选择。

**CAP定理的经典结论是：在存在网络分区的情况下，你必须在一致性和可用性之间做出选择。**



### 拜占庭将军问题

> 在分布式计算中，不同的计算机通过通讯交换信息达成共识而按照同一套协作策略行动。但有时候，系统中的成员计算机可能出错而发送错误的信息，用于传递信息的通讯网络也可能导致信息损坏，使得网络中不同的成员关于全体协作的策略得出不同结论，从而破坏系统一致性。

计算机科学家在 1982 年提出的拜占庭将军问题，说明了这一挑战：

你是一名拜占庭将军，计划攻打一座设防的城市。你有几位盟军将军分布在城市周围，每位将军指挥着自己的军队。为了成功，你必须协调同时发起攻击。如果有些人进攻而另一些人撤退，进攻的部队将被全歼。

你只能通过信使进行通信，而一些将军可能是叛徒，他们希望攻击失败。叛徒可能会：

- 向一些将军发送“进攻”消息，而向另一些将军发送“撤退”消息
- 修改忠诚将军传递的消息
- 与其他叛徒协调以最大化混乱

当你无法区分忠诚的将军和叛徒，也无法信任通信渠道时，如何就“进攻”或“撤退”达成共识？

这似乎是不可能的。几十年来，计算机科学家认为无法构建一个同时具备以下特性的系统：

- 拜占庭容错（即使有恶意参与者也能正常工作）
- 无许可（任何人都可以无需批准加入）
- 去中心化（没有中央权威）

然而在 2008 年，一个自称中本聪的人证明了他们是错的。



### 比特币：第一个区块链

比特币（Bitcoin，BTC）是一种基于区块链的去中心化数字货币，也是第一个真正落地的区块链应用。

虽然其各个组成部分（如加密哈希、数字签名、点对点网络）在此之前已经存在，但中本聪是第一个将它们结合起来，解决数字货币双重支付问题的人。

**区块链**，或称为“区块链条”，正如其在最初的比特币[白皮书](https://bitcoin.org/bitcoin.pdf)中所描述的那样，最终创建了一个同时具备分布式、拜占庭容错和无许可特性的系统。

这一突破并不是试图确定谁值得信任，而是让撒谎在经济上比说真话更昂贵。工作量证明通过要求参与者消耗真实的计算能量来提出更改实现了这一点。攻击者需要在电力上花费的成本超过他们通过攻击所能获得的收益。
