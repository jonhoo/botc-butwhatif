var express = require('express');
var bodyParser = require('body-parser');
var app = express();
app.use(bodyParser.urlencoded({ extended: true }));
app.use(express.static('public'));

var fs = require('fs');
var dbFile = './.data/sqlite.db';
var exists = fs.existsSync(dbFile);
var sqlite3 = require('sqlite3').verbose();
var db = new sqlite3.Database(dbFile);
var async = require('async');

db.serialize(function(){
	if (!exists) {
		db.run('CREATE TABLE Cases (id INTEGER PRIMARY KEY, explanation TEXT NOT NULL, helpful INT NOT NULL DEFAULT 0)');
		db.run('CREATE TABLE Tags (id INTEGER PRIMARY KEY, tag TEXT NOT NULL UNIQUE)');
		db.run('CREATE TABLE Taggings (case_id INT NOT NULL, tag_id INT NOT NULL)');
		db.run('CREATE UNIQUE INDEX tagging ON Taggings(tag_id, case_id)');
	}
});

app.get("/", function (request, response) {
	response.sendFile(__dirname + '/views/index.html');
});

var pick = db.prepare('SELECT id FROM Tags WHERE tag = ?');
app.post('/cases', function(request, response) {
	var tag_ids = [];
	var finished = 0;
	var tags = request.body.query.split(",");
	tags.map(function(v) {
		return v.replace(/[^a-z0-9]/g, "");
	}).forEach(function(tag) {
		pick.all(tag, function(err, rows) {
			if (rows.length == 1) {
				tag_ids.push(rows[0].id);
			} else if (tag.endsWith("s") || tag.endsWith("ed")) {
				// last-ditch effort
				pick.all(tag.replace(/(s|ed)$/, ""), function(err, rows) {
					if (rows.length == 1) {
						tag_ids.push(rows[0].id);
					}

					finished += 1;
					if (finished == tags.length) {
						finalize(response, tag_ids);
					}
				});
				return;
			}

			finished += 1;
			if (finished == tags.length) {
				finalize(response, tag_ids);
			}
		})
	})
});

function finalize(response, tag_ids) {
	if (tag_ids.length == 0) {
		return response.send('[]');
	}

	db.all('SELECT Cases.* ' +
		'FROM Taggings ' +
		'JOIN Cases ON (Cases.id = Taggings.case_id) ' +
		'WHERE Taggings.tag_id IN (' + tag_ids.join(",") + ') ' +
		'GROUP BY Taggings.case_id ' +
		'ORDER BY COUNT(Taggings.tag_id) DESC, Cases.helpful DESC ' +
		'LIMIT 3',
		function(err, rows) {
			var rows = rows.map(function (row) {
				// TODO: boldify
				return row;
			});
			response.send(JSON.stringify(rows));
	});
}

var helpful = db.prepare('UPDATE Cases SET helpful = helpful + 1 WHERE id = ?');
app.post('/helpful', function(request, response) {
	helpful.run([request.body.id], function(err, _) {
		response.send(JSON.stringify(err));
	})
});

var listener = app.listen(process.env.PORT, function () {
	console.log('Your app is listening on port ' + listener.address().port);
});
