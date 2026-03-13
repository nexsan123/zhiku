# edict-005 · 国家资产负债表补全 + 信息源重塑

> 颁发日期：2026-03-12
> 状态：进行中
> 父圣旨：edict-002 (finance-reshaping)

## 旨意

补全信用层"收入端"（国家偿债能力指标），重塑信息源分级体系（T0-T3四层 + AI推理权重），
使推理链从"只看借了多少"升级为"借了多少 × 还得起多少"。

## 新增指标（IMF WEO）

| 指标 | IMF Code | 含义 | 频率 |
|------|----------|------|------|
| GDP 实际增速 | NGDP_RPCH | 收入涨不涨 | 年 |
| 财政赤字/GDP | GGXCNL_NGDP | 每月超支多少 | 年 |
| 经常账户/GDP | BCA_NGDPD | 做生意赚不赚 | 年 |
| 政府总债务/GDP | GGXWDG_NGDP | 总共欠多少 | 年 |
| 政府总收入/GDP | GGR_NGDP | 挣钱能力 | 年 |

## 信息源四层分级

- T0: 国家级权威数据（FRED/BIS/IMF/EIA/央行RSS）→ 权重 1.0
- T1: 国家级通讯社（路透/AP/新华/BBC/半岛）→ 权重 0.8
- T2: 独立财经媒体（FT/WSJ/财新/CNBC）→ 权重 0.6
- T3: 专业/小众源（ZeroHedge/CoinDesk）→ 权重 0.3

## 验收标准

1. IMF WEO API 返回 15 国 × 5 指标真实数据
2. macro_data 表中 IMF_* 记录 ≥ 60 条
3. StatusBar 显示 IMF 状态灯
4. RSS 源 tier 按 T0-T3 重新赋值
5. CreditCyclePanel 显示收入端指标
6. data_reliability.json 包含新指标可靠性
