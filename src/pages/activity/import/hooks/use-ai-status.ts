import { useEffect, useState } from 'react';
import { getAiProviderConfig } from '@/commands/ai';

interface AICapabilities {
  csvMapping: boolean;
  pdfParsing: boolean;
  imageParsing: boolean;
}

export function useAIStatus() {
  const [aiEnabled, setAiEnabled] = useState(false);
  const [provider, setProvider] = useState<string | null>(null);
  const [capabilities, setCapabilities] = useState<AICapabilities>({
    csvMapping: false,
    pdfParsing: false,
    imageParsing: false,
  });

  useEffect(() => {
    async function checkAI() {
      try {
        const config = await getAiProviderConfig();

        if (config.type === 'disabled') {
          setAiEnabled(false);
          setProvider(null);
          setCapabilities({
            csvMapping: false,
            pdfParsing: false,
            imageParsing: false,
          });
          return;
        }

        setAiEnabled(true);
        setProvider(config.type);

        setCapabilities({
          csvMapping: true,  // All providers support this
          pdfParsing: config.type === 'anthropic',
          imageParsing: ['anthropic', 'openai', 'openrouter'].includes(config.type),
        });
      } catch (error) {
        console.error('Failed to check AI status:', error);
        setAiEnabled(false);
      }
    }

    checkAI();
  }, []);

  return { aiEnabled, provider, capabilities };
}
