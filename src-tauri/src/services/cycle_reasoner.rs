use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::ai::{
    CycleIndicators, CycleReasoning, FiveLayerReasoning, ReasoningStep, TurningSignal,
};
use crate::models::credit::{confidence_grade, GlobalCycleOverview};
use crate::services::ai_config::ResolvedAiConfig;
use crate::services::{ai_router, knowledge_base, summarizer};

// ===========================================================================
// Legacy cycle reasoning (kept for backward compat)
// ===========================================================================

/// System prompt for cycle reasoning.
const CYCLE_SYSTEM_PROMPT: &str = r#"你是一位独立的全球金融周期分析师。你站在上帝视角，超越一切国家、政党、意识形态的立场，只用数据和逻辑说话。

核心原则：
- 理性至上：只依据数据事实推理，不受任何政治叙事、媒体情绪、意识形态影响
- 零立场：不站任何国家或阵营，所有政策（无论来自哪国）一律视为"博弈行为"进行客观分析
- 识别偏见：当输入数据中含有政治倾向性信息时，主动标注其偏见方向，剥离情绪后提取事实
- 区分领先/滞后：明确指出哪些指标是领先指标（预测性），哪些是滞后指标（确认性）

给定一组宏观经济和市场指标，输出结构化 JSON 评估。

只回复 JSON 对象（无 markdown，无解释），格式如下：
{
  "cyclePosition": "early_expansion|mid_expansion|late_expansion|recession|recovery",
  "monetaryPolicyStage": "hiking|pausing|cutting|qe|qt",
  "sentimentStage": "panic|fear|caution|neutral|optimism|euphoria",
  "turningSignals": [{"signal": "具体信号描述", "direction": "bullish|bearish", "strength": "weak|moderate|strong"}],
  "sectorRecommendations": ["defensive", "cyclical", "tech"],
  "tailRisks": ["具体风险描述，含触发条件和影响路径"],
  "confidence": 0.0 到 1.0,
  "reasoningChain": "分步推理过程：引用具体数据 → 推导逻辑链 → 得出结论。区分领先指标和滞后指标的信号方向。",
  "timestamp": "ISO 8601 timestamp"
}

规则：
- 用中文回复所有文本字段（reasoningChain、signal、tailRisks 等）
- confidence: 0.0（无信心）到 1.0（高度确信），必须与推理证据强度匹配
- turningSignals: 0-5 个，只在有明确证据时给出
- sectorRecommendations: 2-5 个行业板块
- tailRisks: 1-3 个，必须包含触发条件
- reasoningChain: 4-8 句，必须引用具体数据点，不可泛泛而谈
- 只输出 JSON，不输出任何其他内容

补充指标说明：
- commodities: 大宗商品周期（油/金/铜/天然气价格及趋势），phase 为 inflationary/deflationary/neutral
- crypto: BTC 流动性信号，phase 为 risk_on/risk_off/neutral，反映风险偏好
- fiscal: 美中两国财政锚定（政府债务/GDP、财政收支、GDP增速），来自 IMF WEO
- energy: EIA 能源价格（WTI/Brent 及价差），Brent-WTI 价差反映全球供需结构"#;

/// Run legacy cycle reasoning using configured AI provider.
pub async fn reason_cycle(
    indicators: &CycleIndicators,
    config: &ResolvedAiConfig,
    provider: &str,
) -> Result<CycleReasoning, AppError> {
    if provider != "ollama" && config.api_key.is_empty() {
        log::warn!("AI API key not configured — returning default cycle reasoning");
        return Ok(default_reasoning("No API key configured"));
    }

    let indicators_json = serde_json::to_string_pretty(indicators)
        .map_err(|e| AppError::Parse(format!("Failed to serialize indicators: {}", e)))?;

    let user_prompt = format!(
        "Analyze the following financial cycle indicators and provide your structured assessment:\n\n{}",
        indicators_json
    );

    // Enrich system prompt with country profiles and causal chains
    let system_prompt = format!(
        "{}\n\n=== 知识库 ===\n\n--- 15国结构画像 ---\n{}\n\n--- 结构性因果链 ---\n{}\n\n--- 数据可信度评分 ---\n{}",
        CYCLE_SYSTEM_PROMPT,
        knowledge_base::country_profiles_slim(),
        knowledge_base::power_structures_slim(),
        knowledge_base::DATA_RELIABILITY,
    );

    let response = ai_router::reason(&user_prompt, Some(&system_prompt), config, provider)
        .await?;

    if response.is_empty() {
        return Ok(default_reasoning("Claude returned empty response"));
    }

    parse_reasoning_response(&response)
}

/// Persist a CycleReasoning to ai_analysis.
pub async fn persist_reasoning(
    pool: &SqlitePool,
    reasoning: &CycleReasoning,
    model_label: &str,
) -> Result<(), AppError> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let output_json = serde_json::to_string(reasoning)
        .map_err(|e| AppError::Parse(format!("Failed to serialize reasoning: {}", e)))?;

    sqlx::query(
        r#"INSERT INTO ai_analysis
           (id, analysis_type, input_ids, output, model, confidence, reasoning_chain, source_urls, created_at)
           VALUES (?1, 'cycle_reasoning', NULL, ?2, ?3, ?4, ?5, NULL, ?6)"#,
    )
    .bind(&id)
    .bind(&output_json)
    .bind(model_label)
    .bind(reasoning.confidence)
    .bind(&reasoning.reasoning_chain)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Insert cycle_reasoning failed: {}", e)))?;

    log::info!("Cycle reasoning persisted: id={}, model={}, confidence={:.2}", id, model_label, reasoning.confidence);
    Ok(())
}

/// Fetch the latest CycleReasoning from ai_analysis.
pub async fn get_latest_reasoning(
    pool: &SqlitePool,
) -> Result<Option<CycleReasoning>, AppError> {
    let row: Option<(String,)> = sqlx::query_as(
        r#"SELECT output FROM ai_analysis
           WHERE analysis_type = 'cycle_reasoning'
           ORDER BY created_at DESC LIMIT 1"#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query cycle_reasoning failed: {}", e)))?;

    match row {
        Some((json_str,)) => {
            let reasoning: CycleReasoning = serde_json::from_str(&json_str).map_err(|e| {
                AppError::Parse(format!("Failed to parse stored cycle_reasoning: {}", e))
            })?;
            Ok(Some(reasoning))
        }
        None => Ok(None),
    }
}

fn parse_reasoning_response(response: &str) -> Result<CycleReasoning, AppError> {
    let trimmed = response.trim();

    if let Ok(r) = serde_json::from_str::<CycleReasoning>(trimmed) {
        return Ok(r);
    }

    let stripped = trimmed
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if let Ok(r) = serde_json::from_str::<CycleReasoning>(stripped) {
        return Ok(r);
    }

    if let Some(json_str) = summarizer::extract_json_object(trimmed) {
        if let Ok(r) = serde_json::from_str::<CycleReasoning>(&json_str) {
            return Ok(r);
        }
    }

    Err(AppError::Parse(format!(
        "Failed to parse cycle reasoning: {}",
        &trimmed[..trimmed.len().min(200)]
    )))
}

fn default_reasoning(reason: &str) -> CycleReasoning {
    CycleReasoning {
        cycle_position: "unknown".to_string(),
        monetary_policy_stage: "unknown".to_string(),
        sentiment_stage: "unknown".to_string(),
        turning_signals: Vec::new(),
        sector_recommendations: Vec::new(),
        tail_risks: Vec::new(),
        confidence: 0.0,
        reasoning_chain: reason.to_string(),
        timestamp: Utc::now().to_rfc3339(),
    }
}

// ===========================================================================
// Five-Layer Reasoning (edict-004)
// ===========================================================================

const FIVE_LAYER_SYSTEM_PROMPT: &str = r#"你是一位独立的全球金融情报分析师，精通信用周期、美元潮汐、地缘博弈和市场情绪。你站在上帝视角，超越一切国家、政党、意识形态的立场。

核心原则：
- 理性至上：只依据数据事实推理，不受任何政治叙事、媒体情绪、意识形态影响
- 零立场：不站任何国家或阵营。美国加关税、中国反制、欧盟补贴——一律视为"博弈行为"客观分析其金融影响
- 识别偏见：当数据或新闻含有政治倾向时，主动剥离情绪，只提取可量化的事实
- 交叉验证：同一事件必须从多层视角交叉验证。单一来源的信息降低 confidence
- 区分因果：区分"相关性"和"因果性"。不要因为两件事同时发生就假设因果关系
- 领先 vs 滞后：明确标注哪些是领先指标（收益率曲线、信贷脉冲），哪些是滞后指标（GDP、失业率）

给定五层结构化数据输入，输出综合评估。

只回复 JSON 对象（无 markdown，无解释），格式如下：
{
  "cyclePosition": "early_expansion|mid_expansion|late_expansion|recession|recovery",
  "monetaryPolicyStage": "hiking|pausing|cutting|qe|qt",
  "sentimentStage": "panic|fear|caution|neutral|optimism|euphoria",
  "reasoningSteps": [
    {"step": 1, "layer": "physical", "finding": "能源价格走势分析...", "evidence": ["WTI $72", "天然气持平"], "confidence": 0.8},
    {"step": 2, "layer": "credit", "finding": "全球信用周期正在收紧...", "evidence": ["美国信贷缺口 +5.2pp", "偿债率 14.3%"], "confidence": 0.85},
    {"step": 3, "layer": "dollar", "finding": "美元潮汐状态...", "evidence": ["DXY 3个月 +3.2%", "美联储加息中"], "confidence": 0.7},
    {"step": 4, "layer": "geopolitical", "finding": "地缘博弈态势...", "evidence": ["本周5条关税新闻"], "confidence": 0.6},
    {"step": 5, "layer": "sentiment", "finding": "市场情绪分析...", "evidence": ["恐惧贪婪指数 38", "VIX 22"], "confidence": 0.75}
  ],
  "turningSignals": [{"signal": "具体转折信号", "direction": "bullish|bearish", "strength": "weak|moderate|strong"}],
  "sectorRecommendations": ["defensive", "cyclical", "tech"],
  "tailRisks": ["具体风险描述：触发条件 → 传导路径 → 影响量级"],
  "narrative": "2-3 段专业情报简报风格的叙述，将五层数据串联成完整逻辑链。重点分析各层之间的传导关系和矛盾信号。",
  "confidence": 0.0 到 1.0,
  "timestamp": "ISO 8601 timestamp"
}

规则：
- 用中文回复所有文本字段
- reasoningSteps: 必须恰好 5 步，每层一步（physical, credit, dollar, geopolitical, sentiment）
- 每步必须引用具体数据点作为证据，不可泛泛而谈
- confidence 必须与证据强度匹配：多源交叉验证 ≥ 0.8，单源但可靠 0.5-0.79，推测性 < 0.5
- narrative: 2-3 段，专业情报简报风格，重点分析层间传导和矛盾信号
- turningSignals: 0-5 个转折信号
- sectorRecommendations: 2-5 个行业
- tailRisks: 1-3 个，必须包含触发条件和传导路径
- 只输出 JSON，不输出任何其他内容"#;

/// Input data for five-layer reasoning.
pub struct FiveLayerInput {
    /// Credit layer: GlobalCycleOverview
    pub cycle_overview: GlobalCycleOverview,
    /// Legacy indicators (monetary, economic, market, sentiment, geopolitical)
    pub indicators: CycleIndicators,
    /// Latest deep analysis summaries (from Phase C)
    pub intelligence_summaries: Vec<String>,
    /// Active scenario titles (from Phase D)
    pub active_scenarios: Vec<String>,
}

/// Run five-layer reasoning using configured AI provider.
pub async fn reason_five_layer(
    _pool: &SqlitePool,
    input: &FiveLayerInput,
    config: &ResolvedAiConfig,
    provider: &str,
) -> Result<FiveLayerReasoning, AppError> {
    if provider != "ollama" && config.api_key.is_empty() {
        log::warn!("AI API key not configured — returning default five-layer reasoning");
        return Ok(default_five_layer(&input.cycle_overview, "No API key configured"));
    }

    let prompt = build_five_layer_prompt(input)?;

    // Enrich system prompt with country profiles and causal chains
    let five_layer_enriched = format!(
        "{}\n\n=== 知识库 ===\n\n--- 15国结构画像 ---\n{}\n\n--- 结构性因果链 ---\n{}\n\n--- 数据可信度评分 ---\n{}",
        FIVE_LAYER_SYSTEM_PROMPT,
        knowledge_base::country_profiles_slim(),
        knowledge_base::power_structures_slim(),
        knowledge_base::DATA_RELIABILITY,
    );

    let response =
        ai_router::reason(&prompt, Some(&five_layer_enriched), config, provider).await?;

    if response.is_empty() {
        return Ok(default_five_layer(&input.cycle_overview, "Claude returned empty response"));
    }

    parse_five_layer_response(&response, &input.cycle_overview)
}

/// Persist five-layer reasoning to ai_analysis.
pub async fn persist_five_layer(
    pool: &SqlitePool,
    reasoning: &FiveLayerReasoning,
    model_label: &str,
) -> Result<(), AppError> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let output_json = serde_json::to_string(reasoning)
        .map_err(|e| AppError::Parse(format!("Failed to serialize five-layer reasoning: {}", e)))?;

    sqlx::query(
        r#"INSERT INTO ai_analysis
           (id, analysis_type, input_ids, output, model, confidence, reasoning_chain, source_urls, created_at)
           VALUES (?1, 'five_layer_reasoning', NULL, ?2, ?3, ?4, NULL, NULL, ?5)"#,
    )
    .bind(&id)
    .bind(&output_json)
    .bind(model_label)
    .bind(reasoning.confidence)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Insert five_layer_reasoning failed: {}", e)))?;

    log::info!(
        "Five-layer reasoning persisted: id={}, model={}, confidence={:.2}",
        id,
        model_label,
        reasoning.confidence
    );
    Ok(())
}

/// Fetch the latest five-layer reasoning.
pub async fn get_latest_five_layer(
    pool: &SqlitePool,
) -> Result<Option<FiveLayerReasoning>, AppError> {
    let row: Option<(String,)> = sqlx::query_as(
        r#"SELECT output FROM ai_analysis
           WHERE analysis_type = 'five_layer_reasoning'
           ORDER BY created_at DESC LIMIT 1"#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(format!("Query five_layer_reasoning failed: {}", e)))?;

    match row {
        Some((json_str,)) => {
            let reasoning: FiveLayerReasoning =
                serde_json::from_str(&json_str).map_err(|e| {
                    AppError::Parse(format!("Failed to parse five_layer_reasoning: {}", e))
                })?;
            Ok(Some(reasoning))
        }
        None => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// Five-layer internals
// ---------------------------------------------------------------------------

fn build_five_layer_prompt(input: &FiveLayerInput) -> Result<String, AppError> {
    let mut prompt = String::from("=== FIVE-LAYER DATA INPUT ===\n\n");

    // Layer 1: Physical (real commodity + energy data)
    prompt.push_str("--- LAYER 1: PHYSICAL ---\n");
    let c = &input.indicators.commodities;
    let e = &input.indicators.energy;
    prompt.push_str(&format!(
        "Oil: ${:.2} (trend {:.1}%) | Gold: ${:.2} (trend {:.1}%)\n\
         Copper: ${:.2} (trend {:.1}%) | NatGas: ${:.2} (trend {:.1}%)\n\
         Commodity phase: {}\n\
         EIA: WTI ${:.2}, Brent ${:.2}, spread ${:.2}\n\
         VIX: {:.1}\n",
        c.oil_price, c.oil_trend, c.gold_price, c.gold_trend,
        c.copper_price, c.copper_trend, c.natgas_price, c.natgas_trend,
        c.phase,
        e.wti_price, e.brent_price, e.spread,
        input.indicators.market.vix_level
    ));
    // BTC liquidity signal
    let cr = &input.indicators.crypto;
    prompt.push_str(&format!(
        "BTC: ${:.0} (trend {:.1}%, phase: {})\n",
        cr.btc_price, cr.btc_trend, cr.phase
    ));
    prompt.push_str("\n");

    // Layer 2: Credit
    prompt.push_str("--- LAYER 2: CREDIT ---\n");
    prompt.push_str(&format!(
        "Global Phase: {} ({})\n",
        input.cycle_overview.global_phase_label,
        serde_json::to_string(&input.cycle_overview.global_phase).unwrap_or_default()
    ));
    prompt.push_str(&format!(
        "Global Percentile: {:.0}%\nConfidence: {:.2}\n",
        input.cycle_overview.global_percentile, input.cycle_overview.confidence
    ));

    // Core countries summary
    for c in input
        .cycle_overview
        .countries
        .iter()
        .filter(|c| c.tier == crate::models::credit::CountryTier::Core)
    {
        prompt.push_str(&format!(
            "  {} ({}): {} [gap={}, dsr={}, impulse={}]\n",
            c.country_name,
            c.country_code,
            c.phase_label,
            c.indicators.credit_gdp_gap.map_or("N/A".to_string(), |v| format!("{:.1}", v)),
            c.indicators.debt_service_ratio.map_or("N/A".to_string(), |v| format!("{:.1}", v)),
            c.indicators.credit_impulse.map_or("N/A".to_string(), |v| format!("{:.2}", v)),
        ));
    }

    // Risk alerts
    if !input.cycle_overview.risk_alerts.is_empty() {
        prompt.push_str("Risk Alerts:\n");
        for alert in &input.cycle_overview.risk_alerts {
            prompt.push_str(&format!("  [{}] {}\n", alert.severity, alert.alert));
        }
    }
    // IMF fiscal anchor
    let f = &input.indicators.fiscal;
    prompt.push_str(&format!(
        "Fiscal anchor: US debt/GDP {:.1}%, CN debt/GDP {:.1}%\n\
         Fiscal balance: US {:.1}%, CN {:.1}% | GDP growth: US {:.1}%, CN {:.1}%\n",
        f.us_debt_gdp, f.cn_debt_gdp,
        f.us_fiscal_balance, f.cn_fiscal_balance, f.us_gdp_growth, f.cn_gdp_growth
    ));
    prompt.push_str("\n");

    // Layer 3: Dollar Tide
    prompt.push_str("--- LAYER 3: DOLLAR TIDE ---\n");
    let tide = &input.cycle_overview.dollar_tide;
    prompt.push_str(&format!(
        "State: {} ({})\nDXY 3m: {:.2}%, 6m: {:.2}%\nFed: {}\nM2 growth: {:.2}%\nYield spread: {:.2}pp\n\n",
        tide.tide_label,
        serde_json::to_string(&tide.tide_state).unwrap_or_default(),
        tide.dxy_trend_3m, tide.dxy_trend_6m, tide.fed_policy, tide.m2_growth, tide.yield_spread
    ));

    // Layer 4: Geopolitical + Intelligence
    prompt.push_str("--- LAYER 4: GEOPOLITICAL ---\n");
    prompt.push_str(&format!(
        "Risk level: {}\nEvents (24h): {}\n",
        input.indicators.geopolitical.risk_level, input.indicators.geopolitical.event_count
    ));
    if !input.intelligence_summaries.is_empty() {
        prompt.push_str("Recent intelligence:\n");
        for (i, s) in input.intelligence_summaries.iter().take(5).enumerate() {
            prompt.push_str(&format!("  [{}] {}\n", i + 1, s));
        }
    }
    if !input.active_scenarios.is_empty() {
        prompt.push_str("Active scenarios:\n");
        for s in input.active_scenarios.iter().take(5) {
            prompt.push_str(&format!("  - {}\n", s));
        }
    }
    prompt.push_str("\n");

    // Layer 5: Sentiment
    prompt.push_str("--- LAYER 5: SENTIMENT ---\n");
    prompt.push_str(&format!(
        "Fear & Greed: {:.0}\nNews sentiment avg: {:.2}\nPhase: {}\n",
        input.indicators.sentiment.fear_greed,
        input.indicators.sentiment.news_sentiment_avg,
        input.indicators.sentiment.phase
    ));

    // Monetary context
    prompt.push_str(&format!(
        "\nFed Rate: {:.2}%\nM2 Growth: {:.2}%\nRate Direction: {}\nPolicy Stance: {}\n",
        input.indicators.monetary.fed_rate,
        input.indicators.monetary.m2_growth,
        input.indicators.monetary.rate_direction,
        input.indicators.monetary.policy_stance
    ));

    prompt.push_str("\n=== END DATA ===\n\nProvide your five-layer assessment as JSON:");
    Ok(prompt)
}

fn parse_five_layer_response(
    response: &str,
    overview: &GlobalCycleOverview,
) -> Result<FiveLayerReasoning, AppError> {
    let trimmed = response.trim();

    let parsed: serde_json::Value = if let Ok(v) = serde_json::from_str(trimmed) {
        v
    } else {
        let stripped = trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        if let Ok(v) = serde_json::from_str(stripped) {
            v
        } else if let Some(json_str) = summarizer::extract_json_object(trimmed) {
            serde_json::from_str(&json_str).map_err(|e| {
                AppError::Parse(format!("Five-layer JSON parse failed: {}", e))
            })?
        } else {
            return Err(AppError::Parse(format!(
                "Failed to extract JSON from five-layer response: {}",
                &trimmed[..trimmed.len().min(200)]
            )));
        }
    };

    let conf = parsed.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.3);

    // Parse reasoning steps
    let reasoning_steps: Vec<ReasoningStep> = parsed
        .get("reasoningSteps")
        .or_else(|| parsed.get("reasoning_steps"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| {
                    Some(ReasoningStep {
                        step: s.get("step").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
                        layer: s.get("layer").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                        finding: s.get("finding").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        evidence: s.get("evidence")
                            .and_then(|v| v.as_array())
                            .map(|a| a.iter().filter_map(|e| e.as_str().map(|s| s.to_string())).collect())
                            .unwrap_or_default(),
                        confidence: s.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // Parse turning signals
    let turning_signals: Vec<TurningSignal> = parsed
        .get("turningSignals")
        .or_else(|| parsed.get("turning_signals"))
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    // Parse string arrays
    let sector_recs: Vec<String> = parsed
        .get("sectorRecommendations")
        .or_else(|| parsed.get("sector_recommendations"))
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|s| s.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    let tail_risks: Vec<String> = parsed
        .get("tailRisks")
        .or_else(|| parsed.get("tail_risks"))
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|s| s.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    let risk_alerts: Vec<String> = overview
        .risk_alerts
        .iter()
        .map(|a| format!("[{}] {}", a.severity, a.alert))
        .collect();

    Ok(FiveLayerReasoning {
        global_cycle_phase: serde_json::to_string(&overview.global_phase).unwrap_or_default(),
        global_cycle_phase_zh: overview.global_phase_label.clone(),
        dollar_tide_state: serde_json::to_string(&overview.dollar_tide.tide_state)
            .unwrap_or_default(),
        dollar_tide_label: overview.dollar_tide.tide_label.clone(),
        cycle_position: parsed.get("cyclePosition")
            .or_else(|| parsed.get("cycle_position"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        monetary_policy_stage: parsed.get("monetaryPolicyStage")
            .or_else(|| parsed.get("monetary_policy_stage"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        sentiment_stage: parsed.get("sentimentStage")
            .or_else(|| parsed.get("sentiment_stage"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        reasoning_steps,
        turning_signals,
        sector_recommendations: sector_recs,
        tail_risks,
        risk_alerts,
        confidence: conf,
        confidence_grade: confidence_grade(conf).to_string(),
        narrative: parsed.get("narrative")
            .and_then(|v| v.as_str())
            .unwrap_or("Narrative unavailable")
            .to_string(),
        timestamp: Utc::now().to_rfc3339(),
    })
}

fn default_five_layer(overview: &GlobalCycleOverview, reason: &str) -> FiveLayerReasoning {
    FiveLayerReasoning {
        global_cycle_phase: serde_json::to_string(&overview.global_phase).unwrap_or_default(),
        global_cycle_phase_zh: overview.global_phase_label.clone(),
        dollar_tide_state: serde_json::to_string(&overview.dollar_tide.tide_state)
            .unwrap_or_default(),
        dollar_tide_label: overview.dollar_tide.tide_label.clone(),
        cycle_position: "unknown".to_string(),
        monetary_policy_stage: "unknown".to_string(),
        sentiment_stage: "unknown".to_string(),
        reasoning_steps: Vec::new(),
        turning_signals: Vec::new(),
        sector_recommendations: Vec::new(),
        tail_risks: Vec::new(),
        risk_alerts: overview
            .risk_alerts
            .iter()
            .map(|a| format!("[{}] {}", a.severity, a.alert))
            .collect(),
        confidence: 0.0,
        confidence_grade: "speculative".to_string(),
        narrative: reason.to_string(),
        timestamp: Utc::now().to_rfc3339(),
    }
}
