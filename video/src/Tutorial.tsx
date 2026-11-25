import React from 'react';
import {
  AbsoluteFill,
  Audio,
  Sequence,
  interpolate,
  spring,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
} from 'remotion';
import { Cursor } from './Cursor';

const BG: React.CSSProperties = {
  background: 'radial-gradient(circle at 20% 20%, #1e293b 0%, #090b10 68%)',
  color: '#f8fafc',
  fontFamily: '-apple-system, BlinkMacSystemFont, "SF Pro Display", "Segoe UI", Roboto, sans-serif',
};

const SceneWrap: React.FC<{ children: React.ReactNode; duration: number }> = ({ children, duration }) => {
  const frame = useCurrentFrame();
  const fadeIn = interpolate(frame, [0, 10], [0, 1], { extrapolateRight: 'clamp' });
  const fadeOut = interpolate(frame, [duration - 10, duration], [1, 0], { extrapolateLeft: 'clamp' });
  return (
    <AbsoluteFill style={{ ...BG, opacity: Math.min(fadeIn, fadeOut), padding: 56 }}>
      <div
        style={{
          position: 'absolute',
          width: 440,
          height: 440,
          borderRadius: '50%',
          background: 'rgba(239, 68, 68, 0.14)',
          filter: 'blur(90px)',
          top: -80,
          left: -60,
        }}
      />
      <div
        style={{
          position: 'absolute',
          width: 480,
          height: 480,
          borderRadius: '50%',
          background: 'rgba(37, 99, 235, 0.18)',
          filter: 'blur(100px)',
          bottom: -100,
          right: -60,
        }}
      />
      <div
        style={{
          position: 'relative',
          zIndex: 2,
          width: '100%',
          height: '100%',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        {children}
      </div>
    </AbsoluteFill>
  );
};

const IntroScene: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const scale = spring({ frame, fps, config: { damping: 14 } });
  const textOp = interpolate(frame, [10, 26], [0, 1], { extrapolateRight: 'clamp' });

  return (
    <SceneWrap duration={90}>
      <div style={{ textAlign: 'center', transform: `scale(${scale})` }}>
        <div
          style={{
            width: 96,
            height: 96,
            margin: '0 auto 24px',
            borderRadius: 24,
            background: 'linear-gradient(135deg, #ef4444, #2563eb)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            fontSize: 36,
            fontWeight: 800,
            boxShadow: '0 20px 50px rgba(37, 99, 235, 0.35)',
          }}
        >
          YT
        </div>
        <h1 style={{ fontSize: 64, fontWeight: 800, margin: '0 0 12px', letterSpacing: '-0.03em' }}>
          YT Cloud Saver
        </h1>
        <p style={{ fontSize: 25, color: '#94a3b8', margin: 0, opacity: textOp }}>
          Setup &amp; Usage Guide (3 Simple Steps)
        </p>
      </div>
    </SceneWrap>
  );
};

const Step1DesktopScene: React.FC = () => {
  const frame = useCurrentFrame();
  const isAppPhase = frame >= 75;
  const dlClicked = frame >= 45 && frame <= 58;
  const connectClicked = frame >= 122 && frame <= 134;
  const isConnected = frame >= 136;

  // Coordinates relative to the 500px-wide right card
  const cursorX = !isAppPhase
    ? interpolate(frame, [8, 42], [410, 260], { extrapolateRight: 'clamp' })
    : interpolate(frame, [78, 118], [320, 430], { extrapolateRight: 'clamp' });
  const cursorY = !isAppPhase
    ? interpolate(frame, [8, 42], [230, 82], { extrapolateRight: 'clamp' })
    : interpolate(frame, [78, 118], [260, 152], { extrapolateRight: 'clamp' });

  return (
    <SceneWrap duration={185}>
      <div style={{ width: '100%', display: 'flex', alignItems: 'center', gap: 52 }}>
        <div style={{ flex: 1 }}>
          <div style={{ color: '#60a5fa', fontSize: 15, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.08em', marginBottom: 12 }}>
            Step 1 · Desktop Companion App
          </div>
          <h2 style={{ fontSize: 42, fontWeight: 800, lineHeight: 1.15, margin: '0 0 16px' }}>
            {!isAppPhase ? 'Download Installer from GitHub' : 'Connect Google Drive in 1 Click'}
          </h2>
          <p style={{ fontSize: 19, color: '#94a3b8', lineHeight: 1.5, margin: 0 }}>
            {!isAppPhase
              ? 'Download the Desktop App (.dmg for Mac, .exe for Windows) and yt-cloud-saver-extension.zip from Releases.'
              : 'Open YT Cloud Saver, switch to Cloud & Settings, and click Connect to sign in via browser.'}
          </p>
        </div>

        <div
          style={{
            position: 'relative',
            width: 500,
            background: '#0f1115',
            border: '1px solid #2a303c',
            borderRadius: 16,
            overflow: 'hidden',
            boxShadow: '0 24px 60px rgba(0,0,0,0.65)',
          }}
        >
          {!isAppPhase ? (
            <div style={{ padding: 22 }}>
              <div style={{ fontSize: 15, fontWeight: 700, marginBottom: 14, color: '#e2e8f0' }}>
                GitHub Releases · v1.0.0
              </div>
              {[
                ['YT-Cloud-Saver.dmg (macOS)', '10.4 MB', true],
                ['YT-Cloud-Saver-setup.exe (Windows)', '9.8 MB', false],
                ['yt-cloud-saver-extension.zip', '42 KB', false],
              ].map(([name, size, highlight], idx) => (
                <div
                  key={String(name)}
                  style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                    padding: '12px 14px',
                    borderRadius: 10,
                    background: highlight && dlClicked ? 'rgba(37,99,235,0.28)' : '#171a21',
                    border: `1px solid ${highlight ? '#3b82f6' : '#252a34'}`,
                    marginBottom: idx === 2 ? 0 : 10,
                    fontSize: 14,
                  }}
                >
                  <span>{name}</span>
                  <span style={{ color: '#94a3b8', fontSize: 12 }}>{size}</span>
                </div>
              ))}
            </div>
          ) : (
            <div>
              {/* Faithful replica of desktop/ui/index.html */}
              <div style={{ padding: '12px 16px', borderBottom: '1px solid #222631', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                  <div style={{ width: 30, height: 30, borderRadius: 8, background: 'linear-gradient(135deg, #ef4444, #2563eb)', display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: 11, fontWeight: 700 }}>
                    YT
                  </div>
                  <div>
                    <div style={{ fontSize: 14, fontWeight: 600 }}>YT Cloud Saver</div>
                    <div style={{ fontSize: 11, color: '#9aa3b2' }}>● Ready · Background service active</div>
                  </div>
                </div>
                <div style={{ fontSize: 11, padding: '4px 10px', borderRadius: 999, background: '#171a21', border: '1px solid #2a303c', color: '#9aa3b2' }}>
                  Engine: OK ▾
                </div>
              </div>

              <div style={{ padding: '0 16px', borderBottom: '1px solid #222631', display: 'flex', gap: 16, fontSize: 12 }}>
                <span style={{ padding: '10px 2px', color: '#9aa3b2' }}>Activity</span>
                <span style={{ padding: '10px 2px', color: '#fff', fontWeight: 600, borderBottom: '2px solid #3b82f6' }}>Cloud &amp; Settings</span>
              </div>

              <div style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 10 }}>
                <div style={{ background: '#171a21', border: '1px solid #242934', borderRadius: 10, padding: 12, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                    <div style={{ width: 28, height: 28, borderRadius: '50%', background: isConnected ? 'rgba(34,197,94,0.2)' : 'rgba(59,130,246,0.2)', color: isConnected ? '#4ade80' : '#60a5fa', display: 'flex', alignItems: 'center', justifyContent: 'center', fontWeight: 700, fontSize: 12 }}>
                      G
                    </div>
                    <div>
                      <div style={{ fontSize: 13, fontWeight: 500 }}>Google Drive</div>
                      <div style={{ fontSize: 11, color: isConnected ? '#4ade80' : '#9aa3b2' }}>
                        {isConnected ? 'Connected (gdrive:)' : 'Not connected yet'}
                      </div>
                    </div>
                  </div>
                  <div style={{ padding: '6px 12px', borderRadius: 8, background: connectClicked ? '#1d4ed8' : '#222631', border: '1px solid #323846', fontSize: 12 }}>
                    {isConnected ? 'Reconnect' : 'Connect'}
                  </div>
                </div>

                <div style={{ background: '#171a21', border: '1px solid #242934', borderRadius: 10, padding: 12, fontSize: 12 }}>
                  <div style={{ color: '#9aa3b2', fontSize: 11, marginBottom: 4 }}>Destination Folder</div>
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <span>gdrive:YouTube Saved</span>
                    <span style={{ color: '#60a5fa' }}>Change...</span>
                  </div>
                </div>

                <div style={{ background: '#171a21', border: '1px solid #242934', borderRadius: 10, padding: 12, fontSize: 12 }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
                    <span>Organize into folders by channel</span>
                    <span style={{ color: '#4ade80', fontWeight: 600 }}>✓ ON</span>
                  </div>
                  <div style={{ fontSize: 11, color: '#9aa3b2' }}>Example: gdrive:YouTube Saved/Channel Name/video.mp4</div>
                </div>
              </div>
            </div>
          )}
          <Cursor x={cursorX} y={cursorY} clicking={dlClicked || connectClicked} />
        </div>
      </div>
    </SceneWrap>
  );
};

const Step2ExtensionScene: React.FC = () => {
  const frame = useCurrentFrame();
  const devClicked = frame >= 45 && frame <= 55;
  const devOn = frame >= 50;
  const loadClicked = frame >= 98 && frame <= 110;
  const extLoaded = frame >= 112;

  // Coordinates relative to the 540px-wide chrome://extensions card
  const cursorX = frame < 65
    ? interpolate(frame, [8, 42], [320, 496], { extrapolateRight: 'clamp' })
    : interpolate(frame, [65, 95], [496, 70], { extrapolateRight: 'clamp' });
  const cursorY = frame < 65
    ? interpolate(frame, [8, 42], [180, 22], { extrapolateRight: 'clamp' })
    : interpolate(frame, [65, 95], [22, 78], { extrapolateRight: 'clamp' });

  return (
    <SceneWrap duration={180}>
      <div style={{ width: '100%', display: 'flex', alignItems: 'center', gap: 52 }}>
        <div
          style={{
            position: 'relative',
            width: 540,
            background: '#181a20',
            border: '1px solid #2d3340',
            borderRadius: 16,
            overflow: 'hidden',
            boxShadow: '0 24px 60px rgba(0,0,0,0.65)',
          }}
        >
          <div style={{ padding: '12px 18px', background: '#111318', borderBottom: '1px solid #262b36', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <span style={{ fontSize: 13, color: '#94a3b8' }}>chrome://extensions</span>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 12 }}>
              <span>Developer mode</span>
              <div style={{ width: 34, height: 18, borderRadius: 999, background: devOn ? '#3b82f6' : '#334155', padding: 2, display: 'flex', justifyContent: devOn ? 'flex-end' : 'flex-start' }}>
                <div style={{ width: 14, height: 14, borderRadius: '50%', background: '#fff' }} />
              </div>
            </div>
          </div>

          <div style={{ padding: 20 }}>
            <div style={{ display: 'flex', gap: 10, marginBottom: 18 }}>
              <div
                style={{
                  padding: '8px 14px',
                  borderRadius: 8,
                  background: loadClicked ? '#2563eb' : '#252a34',
                  border: '1px solid #3b82f6',
                  fontSize: 13,
                  fontWeight: 600,
                }}
              >
                Load unpacked
              </div>
              <div style={{ padding: '8px 14px', borderRadius: 8, background: '#1f242d', fontSize: 13, color: '#94a3b8' }}>
                Pack extension
              </div>
            </div>

            {extLoaded ? (
              <div style={{ background: '#111318', border: '1px solid #22c55e', borderRadius: 12, padding: 16 }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 6 }}>
                  <strong style={{ fontSize: 15 }}>YT Cloud Saver 1.0.0</strong>
                  <span style={{ color: '#4ade80', fontSize: 12, fontWeight: 600 }}>Enabled ✓</span>
                </div>
                <div style={{ fontSize: 12, color: '#94a3b8' }}>
                  Save YouTube videos directly to Google Drive or any rclone cloud remote.
                </div>
              </div>
            ) : (
              <div style={{ border: '1px dashed #334155', borderRadius: 12, padding: 26, textAlign: 'center', color: '#64748b', fontSize: 13 }}>
                Select the unzipped <strong>extension/</strong> folder
              </div>
            )}
          </div>
          <Cursor x={cursorX} y={cursorY} clicking={devClicked || loadClicked} />
        </div>

        <div style={{ flex: 1 }}>
          <div style={{ color: '#f59e0b', fontSize: 15, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.08em', marginBottom: 12 }}>
            Step 2 · Chrome Extension
          </div>
          <h2 style={{ fontSize: 42, fontWeight: 800, lineHeight: 1.15, margin: '0 0 16px' }}>
            Load Unpacked in 10 Seconds
          </h2>
          <p style={{ fontSize: 19, color: '#94a3b8', lineHeight: 1.5, margin: 0 }}>
            Unzip <code style={{ color: '#e2e8f0' }}>yt-cloud-saver-extension.zip</code>, turn on Developer mode in Chrome, and select the unzipped folder.
          </p>
        </div>
      </div>
    </SceneWrap>
  );
};

const Step3SaveYouTubeScene: React.FC = () => {
  const frame = useCurrentFrame();
  const clicked = frame >= 44 && frame <= 56;
  const phase = frame < 52 ? 'idle' : frame < 95 ? 'downloading' : frame < 130 ? 'uploading' : 'done';
  const pct = phase === 'downloading'
    ? Math.round(interpolate(frame, [52, 95], [10, 99]))
    : phase === 'uploading'
      ? Math.round(interpolate(frame, [95, 130], [5, 99]))
      : 100;

  // Coordinates relative to the 1020px YouTube card
  const cursorX = interpolate(frame, [10, 42], [640, 815], { extrapolateRight: 'clamp' });
  const cursorY = interpolate(frame, [10, 42], [220, 392], { extrapolateRight: 'clamp' });

  const btnText = phase === 'idle'
    ? 'Save to Cloud'
    : phase === 'downloading'
      ? `Downloading ${pct}%`
      : phase === 'uploading'
        ? `Uploading ${pct}%`
        : 'Saved ✓';

  return (
    <SceneWrap duration={160}>
      <div style={{ width: 1020 }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 14 }}>
          <span style={{ color: '#4ade80', fontSize: 15, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.08em' }}>
            Step 3 · 1-Click Save on YouTube
          </span>
          {phase === 'done' && (
            <div style={{ background: 'rgba(74,222,128,0.16)', border: '1px solid rgba(74,222,128,0.45)', color: '#4ade80', padding: '5px 14px', borderRadius: 999, fontSize: 13, fontWeight: 600 }}>
              Saved to gdrive:YouTube Saved / AI Cinema Studio
            </div>
          )}
        </div>

        <div style={{ position: 'relative', background: '#0f0f0f', border: '1px solid #27272a', borderRadius: 18, overflow: 'hidden', boxShadow: '0 24px 60px rgba(0,0,0,0.7)' }}>
          <div style={{ height: 340, background: 'linear-gradient(135deg, #172554, #09090b 60%, #3b0764)', display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: 22, color: '#cbd5e1' }}>
            ▶ AI Short Film: The Last Horizon (4K)
          </div>
          <div style={{ padding: '18px 24px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <div>
              <div style={{ fontSize: 19, fontWeight: 700, marginBottom: 4 }}>The Last Horizon — Generative AI Film</div>
              <div style={{ fontSize: 13, color: '#a1a1aa' }}>AI Cinema Studio</div>
            </div>
            <div style={{ display: 'flex', gap: 10, alignItems: 'center' }}>
              <div style={{ background: 'rgba(255,255,255,0.1)', padding: '8px 14px', borderRadius: 999, fontSize: 13 }}>👍 1.4K</div>
              <div style={{ background: 'rgba(255,255,255,0.1)', padding: '8px 14px', borderRadius: 999, fontSize: 13 }}>↗ Share</div>
              {/* Exact replica of extension/content.js button */}
              <div
                style={{
                  position: 'relative',
                  overflow: 'hidden',
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: 6,
                  height: 36,
                  padding: '0 14px',
                  borderRadius: 18,
                  fontSize: 14,
                  fontWeight: 500,
                  background: phase === 'done' ? 'rgba(43,166,64,0.2)' : phase === 'idle' ? 'rgba(255,255,255,0.1)' : 'rgba(62,166,255,0.18)',
                  color: phase === 'done' ? '#4ade80' : phase === 'idle' ? '#f1f1f1' : '#3ea6ff',
                }}
              >
                <span>☁</span>
                <span>{btnText}</span>
                {phase !== 'idle' && (
                  <div style={{ position: 'absolute', left: 0, bottom: 0, height: 3, width: `${pct}%`, background: phase === 'done' ? '#4ade80' : '#3ea6ff' }} />
                )}
              </div>
              <div style={{ background: 'rgba(255,255,255,0.1)', padding: '8px 14px', borderRadius: 999, fontSize: 13 }}>↓ Download</div>
            </div>
          </div>
          <Cursor x={cursorX} y={cursorY} clicking={clicked} />
        </div>
      </div>
    </SceneWrap>
  );
};

const Step4FallbackScene: React.FC = () => {
  const frame = useCurrentFrame();
  const switched = frame >= 46;
  const playWidth = interpolate(frame, [52, 130], [8, 65], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' });

  return (
    <SceneWrap duration={135}>
      <div style={{ width: '100%', textAlign: 'center' }}>
        <div style={{ color: '#60a5fa', fontSize: 15, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.08em', marginBottom: 10 }}>
          Protected Against Removal
        </div>
        <h2 style={{ fontSize: 40, fontWeight: 800, margin: '0 0 22px' }}>
          Auto-plays from Google Drive if YouTube removes it
        </h2>

        <div
          style={{
            width: 820,
            height: 340,
            margin: '0 auto',
            borderRadius: 18,
            overflow: 'hidden',
            border: '1px solid #27272a',
            background: switched ? 'linear-gradient(135deg, #0f172a, #1e1b4b)' : '#09090b',
            position: 'relative',
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          {!switched ? (
            <>
              <div style={{ fontSize: 38, marginBottom: 10 }}>⚠️</div>
              <div style={{ fontSize: 20, fontWeight: 600 }}>Video unavailable</div>
              <div style={{ fontSize: 14, color: '#71717a', marginTop: 6 }}>This video is no longer available due to a copyright claim.</div>
            </>
          ) : (
            <>
              <div style={{ position: 'absolute', top: 14, right: 14, background: 'rgba(15,17,21,0.85)', border: '1px solid rgba(74,222,128,0.45)', color: '#4ade80', padding: '5px 12px', borderRadius: 999, fontSize: 12, fontWeight: 600 }}>
                Playing saved copy from Google Drive
              </div>
              <div style={{ fontSize: 24, fontWeight: 600, color: '#e2e8f0' }}>Streaming straight from Google Drive</div>
              <div style={{ position: 'absolute', left: 20, right: 20, bottom: 18, height: 6, borderRadius: 999, background: 'rgba(255,255,255,0.2)' }}>
                <div style={{ width: `${playWidth}%`, height: '100%', borderRadius: 999, background: '#4ade80' }} />
              </div>
            </>
          )}
        </div>
      </div>
    </SceneWrap>
  );
};

export const Tutorial: React.FC = () => {
  return (
    <AbsoluteFill>
      <Audio src={staticFile('bgm.wav')} volume={0.6} />
      <Sequence from={0} durationInFrames={90}>
        <IntroScene />
      </Sequence>
      <Sequence from={90} durationInFrames={185}>
        <Step1DesktopScene />
      </Sequence>
      <Sequence from={275} durationInFrames={180}>
        <Step2ExtensionScene />
      </Sequence>
      <Sequence from={455} durationInFrames={160}>
        <Step3SaveYouTubeScene />
      </Sequence>
      <Sequence from={615} durationInFrames={135}>
        <Step4FallbackScene />
      </Sequence>
    </AbsoluteFill>
  );
};
