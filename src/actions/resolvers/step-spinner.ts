import ora from 'ora';

const loadStepSpinner = async (
  steps: {
    startTitle: string;
    stepAction: () => Promise<string>;
    errorAction: (() => Promise<void>) | null;
    finallyAction: (() => Promise<void>) | null;
  }[]
) => {
  for (const step of steps) {
    const spinner = ora({
      text: step.startTitle,
      color: 'yellow'
    }).start();

    try {
      spinner.succeed(await step.stepAction());
    } catch (error: any) {
      if (step.errorAction) {
        await step.errorAction();
      }
      spinner.fail(`[BREAK] ${error}`);
      break;
    } finally {
      if (step.finallyAction) {
        await step.finallyAction();
      }
    }
  }
};

export { loadStepSpinner };
