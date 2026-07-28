import { useState } from "react";
import {
  ArrowRight,
  CheckCircle2,
  Command,
  RotateCcw,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import type { SettingsInput, StartupPolicy } from "../types";

interface OnboardingProps {
  busy: boolean;
  onComplete: (settings: SettingsInput) => Promise<void>;
}

const policies: Array<{
  value: StartupPolicy;
  title: string;
  description: string;
}> = [
  {
    value: "ask",
    title: "確認してから復元",
    description: "前回動いていたアプリを一覧にし、選んで起動します。",
  },
  {
    value: "auto",
    title: "自動で復元",
    description: "自動復元に指定したプロジェクトをすぐ起動します。",
  },
  {
    value: "manual",
    title: "常に手動",
    description: "状態だけ記録し、プロジェクトは自動で起動しません。",
  },
];

export function Onboarding({ busy, onComplete }: OnboardingProps) {
  const [step, setStep] = useState(0);
  const [launchAtLogin, setLaunchAtLogin] = useState(true);
  const [policy, setPolicy] = useState<StartupPolicy>("ask");

  return (
    <main className="onboarding-shell">
      <div className="onboarding-orb onboarding-orb-one" />
      <div className="onboarding-orb onboarding-orb-two" />
      <section className="onboarding-card">
        <div className="onboarding-brand">
          <div className="brand-mark">
            <Command size={22} strokeWidth={2.4} />
            <span />
          </div>
          <strong>Vibe Manager</strong>
          <span className="beta-chip">LOCAL</span>
        </div>

        <div className="step-dots" aria-label={`セットアップ ${step + 1}/3`}>
          {[0, 1, 2].map((index) => (
            <span key={index} className={index <= step ? "active" : ""} />
          ))}
        </div>

        {step === 0 && (
          <div className="onboarding-content">
            <div className="hero-icon">
              <Sparkles size={28} />
            </div>
            <p className="eyebrow">WELCOME</p>
            <h1>
              ローカルアプリを、
              <br />
              もう見失わない。
            </h1>
            <p className="onboarding-copy">
              localhostで動く開発サーバーを見つけ、一か所で起動・停止・監視。
              PC再起動後も、何が動いていたかを覚えています。
            </p>
            <div className="feature-row">
              <span>
                <CheckCircle2 size={16} /> 状態を記録
              </span>
              <span>
                <CheckCircle2 size={16} /> 自動検出
              </span>
              <span>
                <CheckCircle2 size={16} /> 安全に復元
              </span>
            </div>
          </div>
        )}

        {step === 1 && (
          <div className="onboarding-content align-left">
            <div className="hero-icon small">
              <ShieldCheck size={25} />
            </div>
            <p className="eyebrow">STARTUP</p>
            <h1>Vibe Managerの起動</h1>
            <p className="onboarding-copy">
              ログイン後にトレイ／メニューバーへ常駐させると、
              いつでも現在の状態を確認できます。
            </p>
            <label className="choice-card toggle-choice">
              <div>
                <strong>ログイン時に自動起動</strong>
                <span>画面は開かず、トレイに静かに常駐します。</span>
              </div>
              <input
                type="checkbox"
                checked={launchAtLogin}
                onChange={(event) => setLaunchAtLogin(event.target.checked)}
              />
              <span className="switch" />
            </label>
            <p className="hint">この設定はあとから設定画面で変更できます。</p>
          </div>
        )}

        {step === 2 && (
          <div className="onboarding-content align-left">
            <div className="hero-icon small">
              <RotateCcw size={24} />
            </div>
            <p className="eyebrow">RESTORE</p>
            <h1>標準の復元方法</h1>
            <p className="onboarding-copy">
              新しく登録するプロジェクトの初期設定を選びます。
              プロジェクトごとに変更できます。
            </p>
            <div className="policy-list">
              {policies.map((option) => (
                <button
                  key={option.value}
                  type="button"
                  className={`choice-card policy-choice ${
                    policy === option.value ? "selected" : ""
                  }`}
                  onClick={() => setPolicy(option.value)}
                >
                  <span className="radio">
                    {policy === option.value && <span />}
                  </span>
                  <span>
                    <strong>{option.title}</strong>
                    <small>{option.description}</small>
                  </span>
                </button>
              ))}
            </div>
          </div>
        )}

        <footer className="onboarding-footer">
          {step > 0 ? (
            <button
              type="button"
              className="text-button"
              onClick={() => setStep((current) => current - 1)}
            >
              戻る
            </button>
          ) : (
            <span />
          )}
          {step < 2 ? (
            <button
              type="button"
              className="primary-button"
              onClick={() => setStep((current) => current + 1)}
            >
              続ける <ArrowRight size={17} />
            </button>
          ) : (
            <button
              type="button"
              className="primary-button"
              disabled={busy}
              onClick={() =>
                void onComplete({
                  onboardingComplete: true,
                  launchAtLogin,
                  defaultStartupPolicy: policy,
                  discoveryEnabled: true,
                  autoRegisterDiscovered: false,
                  workspaceRoots: [],
                })
              }
            >
              {busy ? "設定しています…" : "セットアップを完了"}
              {!busy && <ArrowRight size={17} />}
            </button>
          )}
        </footer>
      </section>
    </main>
  );
}
