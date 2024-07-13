import figlet from 'figlet';

const showGraphic = () => {
  console.log();
  console.log(
    figlet.textSync('_UUPM_', {
      font: 'Ghost',
      horizontalLayout: 'default',
      verticalLayout: 'default',
      whitespaceBreak: true
    })
  );
  console.log();
};

export default showGraphic;
