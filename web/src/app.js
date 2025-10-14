import './app.scss';

function updateButton() {
    const button = document.getElementsByTagName('button')[0];
    const selects = document.getElementsByTagName('select');

    for (const select of selects) {
        if (!select.value) {
            button.setAttribute('disabled', 'disabled');
            return;
        }
    }

    button.removeAttribute('disabled');
}

document.forms[0].onchange = updateButton;

updateButton();
