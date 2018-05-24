(function(){
	const caseList = document.getElementById('cases');
	const caseForm = document.forms[0];
	const caseQuery = caseForm.elements['query'];

	const onCases = function() {
		// parse our response to convert to JSON
		const cases = JSON.parse(this.responseText);
		caseList.innerHTML = "";

		caseForm.classList.remove("loading");
		caseList.classList.remove("loading");

		// iterate through every dream and add it to our page
		cases.forEach( function(c) {
			const one = document.createElement('article');
			one.innerHTML = '<p>' + c.explanation + '</p>';

			const oneup = document.createElement('a');
			oneup.classList.add('good');
			oneup.textContent = '✓';
			oneup.setAttribute('title', 'This was helpful');
			oneup.addEventListener('click', function(e) {
				e.preventDefault();

				oneup.classList.add('voted');

				const cases = new XMLHttpRequest();
				cases.open('POST', '/helpful', true);
				cases.setRequestHeader("Content-Type", "application/x-www-form-urlencoded");
				cases.send("id=" + encodeURIComponent(c.id));
			});
			one.prepend(oneup);

			if (c.helpful != 0) {
				const helpful = document.createElement('footer');
				helpful.classList.add('helpful');
				helpful.textContent = 'This example has been helpful to ' + c.helpful + ' other players.';
				one.appendChild(helpful);
			}
			caseList.appendChild(one);
		});
	}

	// listen for the form to be submitted and add a new dream when it is
	caseForm.onsubmit = function(event) {
		// stop our form submission from refreshing the page
		event.preventDefault();

		caseForm.classList.add("loading");
		caseList.classList.add("loading");

		// get dream value and add it to the list
		const cases = new XMLHttpRequest();
		cases.onload = onCases;
		cases.open('POST', '/cases', true);
		cases.setRequestHeader("Content-Type", "application/x-www-form-urlencoded");
		cases.send("query=" + encodeURIComponent(caseQuery.value));
	};

})()
